//! The page/block graph: resolution of references, backlinks, inherited references,
//! linked references, tasks, namespaces and co-mentions.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::ops::Range;
use std::path::{Path, PathBuf};

use crate::diag::{Diagnostic, DiagnosticKind};
use crate::layout::{
    self, Address, Date, JOURNALS_DIR, NameKey, PAGES_DIR, SourceFile, SourceKind, normalize_name,
    percent_decode, validate_name,
};
use crate::parse::{self, BlockKind, ParsedBlock, Property, RawRef, RawRefKind, TaskState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PageId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageKind {
    Journal(Date),
    Regular,
}

#[derive(Debug, Clone)]
pub struct Page {
    /// Display name.
    pub name: String,
    pub key: NameKey,
    pub kind: PageKind,
    /// Source file relative to the root; `None` for implicit pages.
    pub file: Option<PathBuf>,
    /// Top-level blocks in document order.
    pub roots: Vec<BlockId>,
    pub ns_parent: Option<PageId>,
    /// Direct namespace children, ordered by key.
    pub ns_children: Vec<PageId>,
    /// Accepted names from the page's `alias` property, normalized, in written order.
    pub aliases: Vec<String>,
}

impl Page {
    pub fn is_implicit(&self) -> bool {
        self.file.is_none()
    }

    pub fn is_journal(&self) -> bool {
        matches!(self.kind, PageKind::Journal(_))
    }

    pub fn date(&self) -> Option<Date> {
        match self.kind {
            PageKind::Journal(date) => Some(date),
            PageKind::Regular => None,
        }
    }

    /// Last namespace segment of the name.
    pub fn leaf_name(&self) -> &str {
        self.name.rsplit('/').next().unwrap_or(&self.name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefTarget {
    Page(PageId),
    Block(BlockId),
    /// Dangling block ref or markdown link to a missing file; display as broken.
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefLocation {
    /// In the block's `content`.
    Content,
    /// In the value of the block's property at this index.
    Property(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ref {
    pub kind: RawRefKind,
    /// Page name, block id, or link destination as written.
    pub text: String,
    /// Byte range of the reference syntax within its location's text.
    pub range: Range<usize>,
    /// For markdown links: byte range of the destination token within the same text.
    pub dest: Option<Range<usize>>,
    pub location: RefLocation,
    pub target: RefTarget,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub page: PageId,
    /// 0-based position among its page's blocks, in document order.
    pub index: u32,
    pub parent: Option<BlockId>,
    pub children: Vec<BlockId>,
    pub kind: BlockKind,
    /// 1-based line of the block's first non-blank line.
    pub line: u32,
    /// Display content without properties and task marker.
    pub content: String,
    pub props: Vec<Property>,
    pub id: Option<String>,
    pub task: Option<TaskState>,
    pub refs: Vec<Ref>,
}

impl Block {
    pub fn prop(&self, key: &str) -> Option<&str> {
        self.props
            .iter()
            .find(|p| p.key == key)
            .map(|p| p.value.as_str())
    }

    /// Whether this block holds its page's properties: the page's first block, made
    /// only of property lines.
    pub fn holds_page_props(&self) -> bool {
        self.index == 0 && parse::is_properties_only(&self.content, self.task, &self.props)
    }
}

/// A linked reference: a block referencing the page, with its ancestors (root first).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedRef {
    pub block: BlockId,
    pub breadcrumb: Vec<BlockId>,
}

/// Linked references coming from one source page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefGroup {
    pub page: PageId,
    pub refs: Vec<LinkedRef>,
}

/// A page co-mentioned with another page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Related<'g> {
    pub page: PageId,
    /// Blocks mentioning both pages, in scan order.
    pub blocks: &'g [BlockId],
    /// Newest journal date among `blocks`.
    pub recency: Option<Date>,
}

impl Related<'_> {
    pub fn weight(&self) -> usize {
        self.blocks.len()
    }
}

/// The whole-wiki graph of the graph view: every non-journal page, and an edge between
/// two of them for each block connecting them (a link from one to the other, or a
/// co-mention).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Overview {
    /// Nodes in page order.
    pub nodes: Vec<PageId>,
    /// Per node: the number of other pages with a block directly referencing it.
    pub mentions: Vec<u32>,
    /// `(node, node, weight)` by node index, smaller index first, sorted.
    pub edges: Vec<(u32, u32, u32)>,
}

#[derive(Debug, Clone, Default)]
pub struct Graph {
    root: PathBuf,
    pages: Vec<Page>,
    blocks: Vec<Block>,
    by_key: HashMap<NameKey, PageId>,
    by_id: HashMap<String, BlockId>,
    /// Alias names of pages; never the name of a page with a file.
    aliases: HashMap<NameKey, PageId>,
    /// Per page: blocks that directly reference the page or one of its blocks.
    backlinks: Vec<Vec<BlockId>>,
    /// Per page: task blocks on the page or inheriting a reference to it.
    tasks: Vec<Vec<BlockId>>,
    /// Co-mentions keyed by `(smaller, larger)` page id.
    comentions: HashMap<(PageId, PageId), Vec<BlockId>>,
    /// Per page: co-mentioned pages.
    related: Vec<Vec<PageId>>,
}

impl Graph {
    /// Loads the notes directory at `root`. Never fails on content: problems are returned
    /// as diagnostics. The same directory contents always produce the same result.
    pub fn load(root: &Path) -> (Graph, Vec<Diagnostic>) {
        let mut diags = Vec::new();
        let files = layout::scan(root, &mut diags);
        let loaded = read_and_parse(root, files, &mut diags);
        warn_pages_index(&loaded, &mut diags);
        let keep = pick_colliding_winners(&loaded, &mut diags);

        let mut g = Graph {
            root: root.to_path_buf(),
            ..Graph::default()
        };
        let mut file_pages = Vec::new();
        for (l, _) in loaded.iter().zip(&keep).filter(|(_, k)| **k) {
            let kind = match l.file.kind {
                SourceKind::Journal(date) => PageKind::Journal(date),
                SourceKind::Regular => PageKind::Regular,
            };
            let id = g.add_page(&l.file.name, kind, Some(l.file.rel.clone()));
            file_pages.push((id, l));
        }

        let mut raw_refs: Vec<Vec<(RawRef, RefLocation)>> = Vec::new();
        for (page, l) in &file_pages {
            g.add_blocks(*page, l, &mut raw_refs, &mut diags);
        }
        g.build_id_table(&mut diags);
        g.build_aliases(&mut diags);

        let mut resolver = Resolver { diags: &mut diags };
        for (page, l) in &file_pages {
            let name = g.pages[page.0 as usize].name.clone();
            g.ensure_namespace_prefixes(&name);
            let first = g.pages[page.0 as usize].roots.first().copied();
            let Some(first) = first else { continue };
            // Blocks of one file are contiguous, starting at the first root.
            let count = l.blocks.len();
            for b in first.0..first.0 + count as u32 {
                let refs = std::mem::take(&mut raw_refs[b as usize]);
                for (raw, location) in refs {
                    if let Some(r) =
                        resolver.resolve(&mut g, BlockId(b), &l.file.rel, raw, location)
                    {
                        g.blocks[b as usize].refs.push(r);
                    }
                }
            }
        }

        g.link_namespaces();
        g.build_backlinks();
        g.build_tasks();
        g.build_comentions();
        (g, diags)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn pages(&self) -> impl Iterator<Item = (PageId, &Page)> {
        self.pages
            .iter()
            .enumerate()
            .map(|(i, p)| (PageId(i as u32), p))
    }

    pub fn page(&self, id: PageId) -> &Page {
        &self.pages[id.0 as usize]
    }

    pub fn blocks(&self) -> impl Iterator<Item = (BlockId, &Block)> {
        self.blocks
            .iter()
            .enumerate()
            .map(|(i, b)| (BlockId(i as u32), b))
    }

    pub fn block(&self, id: BlockId) -> &Block {
        &self.blocks[id.0 as usize]
    }

    /// Finds a page by name, case-insensitively.
    pub fn find_page(&self, name: &str) -> Option<PageId> {
        self.by_key.get(&NameKey::new(name)).copied()
    }

    /// The page's properties: those of its first block when that block is made only of
    /// property lines.
    pub fn page_props(&self, page: PageId) -> &[Property] {
        match self.page(page).roots.first().map(|&b| self.block(b)) {
            Some(b) if b.holds_page_props() => &b.props,
            _ => &[],
        }
    }

    /// Finds a block by its `id::` property.
    pub fn find_block(&self, id: &str) -> Option<BlockId> {
        self.by_id.get(id).copied()
    }

    /// Ancestors of a block, root first.
    pub fn ancestors(&self, block: BlockId) -> Vec<BlockId> {
        let mut out = Vec::new();
        let mut cur = self.block(block).parent;
        while let Some(b) = cur {
            out.push(b);
            cur = self.block(b).parent;
        }
        out.reverse();
        out
    }

    /// Pages a block directly references (block refs count as their target's page).
    pub fn direct_pages(&self, block: BlockId) -> BTreeSet<PageId> {
        self.block(block)
            .refs
            .iter()
            .filter_map(|r| self.target_page(r.target))
            .collect()
    }

    /// Resolved references of the block and all its ancestors.
    pub fn inherited_targets(&self, block: BlockId) -> Vec<RefTarget> {
        let mut out = Vec::new();
        let mut cur = Some(block);
        while let Some(b) = cur {
            out.extend(
                self.block(b)
                    .refs
                    .iter()
                    .map(|r| r.target)
                    .filter(|t| *t != RefTarget::Unresolved),
            );
            cur = self.block(b).parent;
        }
        out
    }

    /// Pages the block or any ancestor references (block refs count as their page).
    pub fn inherited_pages(&self, block: BlockId) -> BTreeSet<PageId> {
        self.inherited_targets(block)
            .into_iter()
            .filter_map(|t| self.target_page(t))
            .collect()
    }

    /// Blocks directly referencing the page or one of its blocks, in scan order.
    pub fn backlinks(&self, page: PageId) -> &[BlockId] {
        &self.backlinks[page.0 as usize]
    }

    /// Linked references of a page, grouped by source page and ordered: journals newest
    /// first, then regular pages by case-insensitive name.
    pub fn linked_references(&self, page: PageId) -> Vec<RefGroup> {
        let candidates: HashSet<BlockId> = self
            .backlinks(page)
            .iter()
            .copied()
            .filter(|&b| self.block(b).page != page)
            .collect();
        let mut groups: Vec<RefGroup> = Vec::new();
        for &b in self.backlinks(page) {
            if !candidates.contains(&b) {
                continue;
            }
            let breadcrumb = self.ancestors(b);
            if breadcrumb.iter().any(|a| candidates.contains(a)) {
                continue;
            }
            let source = self.block(b).page;
            let linked = LinkedRef {
                block: b,
                breadcrumb,
            };
            match groups.last_mut() {
                Some(group) if group.page == source => group.refs.push(linked),
                _ => groups.push(RefGroup {
                    page: source,
                    refs: vec![linked],
                }),
            }
        }
        groups.sort_by(|a, b| {
            let (pa, pb) = (self.page(a.page), self.page(b.page));
            match (pa.date(), pb.date()) {
                (Some(da), Some(db)) => db.cmp(&da),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => display_order(&pa.name, &pb.name),
            }
        });
        groups
    }

    /// Task blocks on the page or inheriting a reference to it, in scan order.
    pub fn page_tasks(&self, page: PageId) -> &[BlockId] {
        &self.tasks[page.0 as usize]
    }

    /// Open (TODO, DOING) tasks of a page.
    pub fn open_page_tasks(&self, page: PageId) -> Vec<BlockId> {
        self.page_tasks(page)
            .iter()
            .copied()
            .filter(|&b| self.is_open_task(b))
            .collect()
    }

    /// All open tasks in the graph, in scan order.
    pub fn open_tasks(&self) -> Vec<BlockId> {
        self.blocks()
            .map(|(id, _)| id)
            .filter(|&b| self.is_open_task(b))
            .collect()
    }

    /// Direct namespace children of a page, ordered by key.
    pub fn namespace_children(&self, page: PageId) -> &[PageId] {
        &self.page(page).ns_children
    }

    /// Pages co-mentioned with `page`, ordered by weight (desc), recency (newest first),
    /// then case-insensitive name.
    pub fn related(&self, page: PageId) -> Vec<Related<'_>> {
        let mut out: Vec<Related<'_>> = self.related[page.0 as usize]
            .iter()
            .map(|&other| {
                let key = (page.min(other), page.max(other));
                let blocks = self.comentions[&key].as_slice();
                let recency = blocks
                    .iter()
                    .filter_map(|&b| self.page(self.block(b).page).date())
                    .max();
                Related {
                    page: other,
                    blocks,
                    recency,
                }
            })
            .collect();
        out.sort_by(|a, b| {
            b.weight()
                .cmp(&a.weight())
                .then_with(|| b.recency.cmp(&a.recency))
                .then_with(|| display_order(&self.page(a.page).name, &self.page(b.page).name))
        });
        out
    }

    /// The graph of all non-journal pages; see [`Overview`]. The page `index` is left
    /// out when it has no file, as it then has no page of its own.
    pub fn overview(&self) -> Overview {
        let mut index = vec![u32::MAX; self.pages.len()];
        let mut nodes = Vec::new();
        for (id, page) in self.pages() {
            if !page.is_journal() && !(page.is_implicit() && layout::is_home_name(&page.name)) {
                index[id.0 as usize] = nodes.len() as u32;
                nodes.push(id);
            }
        }
        let mentions = nodes
            .iter()
            .map(|&p| {
                let sources: HashSet<PageId> = self
                    .backlinks(p)
                    .iter()
                    .map(|&b| self.block(b).page)
                    .filter(|&s| s != p)
                    .collect();
                sources.len() as u32
            })
            .collect();
        let mut weights: BTreeMap<(u32, u32), u32> = BTreeMap::new();
        let mut pairs = BTreeSet::new();
        for (id, block) in self.blocks() {
            let here = index[block.page.0 as usize];
            let refs: Vec<u32> = self
                .direct_pages(id)
                .into_iter()
                .map(|p| index[p.0 as usize])
                .filter(|&n| n != u32::MAX && n != here)
                .collect();
            pairs.clear();
            for (i, &a) in refs.iter().enumerate() {
                if here != u32::MAX {
                    pairs.insert((here.min(a), here.max(a)));
                }
                for &b in &refs[i + 1..] {
                    pairs.insert((a.min(b), a.max(b)));
                }
            }
            for &pair in &pairs {
                *weights.entry(pair).or_default() += 1;
            }
        }
        Overview {
            nodes,
            mentions,
            edges: weights.into_iter().map(|((a, b), w)| (a, b, w)).collect(),
        }
    }

    // -- building -----------------------------------------------------------------

    fn is_open_task(&self, block: BlockId) -> bool {
        self.block(block).task.is_some_and(TaskState::is_open)
    }

    fn target_page(&self, target: RefTarget) -> Option<PageId> {
        match target {
            RefTarget::Page(p) => Some(p),
            RefTarget::Block(b) => Some(self.block(b).page),
            RefTarget::Unresolved => None,
        }
    }

    fn add_page(&mut self, name: &str, kind: PageKind, file: Option<PathBuf>) -> PageId {
        let id = PageId(self.pages.len() as u32);
        let key = NameKey::new(name);
        self.by_key.insert(key.clone(), id);
        self.pages.push(Page {
            name: name.to_owned(),
            key,
            kind,
            file,
            roots: Vec::new(),
            ns_parent: None,
            ns_children: Vec::new(),
            aliases: Vec::new(),
        });
        id
    }

    /// Looks up a page by (valid, normalized) name, creating an implicit page if needed.
    fn page_for_name(&mut self, name: &str) -> PageId {
        if let Some(&id) = self.by_key.get(&NameKey::new(name)) {
            return id;
        }
        let kind = Date::parse(name).map_or(PageKind::Regular, PageKind::Journal);
        let id = self.add_page(name, kind, None);
        self.ensure_namespace_prefixes(name);
        id
    }

    fn ensure_namespace_prefixes(&mut self, name: &str) {
        let mut end = 0;
        while let Some(slash) = name[end..].find('/') {
            end += slash;
            self.page_for_name(&name[..end]);
            end += 1;
        }
    }

    fn add_blocks(
        &mut self,
        page: PageId,
        l: &Loaded,
        raw_refs: &mut Vec<Vec<(RawRef, RefLocation)>>,
        diags: &mut Vec<Diagnostic>,
    ) {
        let base = self.blocks.len() as u32;
        for pb in &l.blocks {
            let id = BlockId(self.blocks.len() as u32);
            let parent = pb.parent.map(|p| BlockId(base + p as u32));
            let block_id = pb.prop("id").and_then(|value| {
                if parse::is_valid_block_id(value) {
                    Some(value.to_owned())
                } else {
                    diags.push(Diagnostic::new(
                        DiagnosticKind::InvalidBlockId,
                        l.file.rel.clone(),
                        Some(pb.line),
                        format!("invalid block id `{value}`: must be non-empty, without whitespace or parentheses"),
                    ));
                    None
                }
            });
            let mut refs: Vec<(RawRef, RefLocation)> = pb
                .refs
                .iter()
                .map(|r| (r.clone(), RefLocation::Content))
                .collect();
            let page_props =
                id.0 == base && parse::is_properties_only(&pb.content, pb.task, &pb.props);
            for (i, prop) in pb.props.iter().enumerate() {
                if page_props && prop.key == "alias" {
                    // Links among the aliases would only point back at this page.
                    continue;
                }
                refs.extend(
                    prop.refs
                        .iter()
                        .map(|r| (r.clone(), RefLocation::Property(i))),
                );
            }
            raw_refs.push(refs);
            self.blocks.push(Block {
                page,
                index: id.0 - base,
                parent,
                children: Vec::new(),
                kind: pb.kind,
                line: pb.line,
                content: pb.content.clone(),
                props: pb.props.clone(),
                id: block_id,
                task: pb.task,
                refs: Vec::new(),
            });
            match parent {
                Some(p) => self.blocks[p.0 as usize].children.push(id),
                None => self.pages[page.0 as usize].roots.push(id),
            }
        }
    }

    fn build_id_table(&mut self, diags: &mut Vec<Diagnostic>) {
        let mut seen: HashMap<String, Vec<BlockId>> = HashMap::new();
        let mut order = Vec::new();
        for (i, block) in self.blocks.iter().enumerate() {
            if let Some(id) = &block.id {
                let entry = seen.entry(id.clone()).or_default();
                if entry.is_empty() {
                    order.push(id.clone());
                }
                entry.push(BlockId(i as u32));
            }
        }
        for id in order {
            let blocks = &seen[&id];
            self.by_id.insert(id.clone(), blocks[0]);
            if blocks.len() > 1 {
                let locations: Vec<String> = blocks.iter().map(|&b| self.location(b)).collect();
                let first = self.block(blocks[0]);
                diags.push(Diagnostic::new(
                    DiagnosticKind::DuplicateBlockId,
                    self.page(first.page).file.clone().unwrap_or_default(),
                    Some(first.line),
                    format!(
                        "block id `{id}` is used {} times ({}); the first one is used",
                        blocks.len(),
                        locations.join(", ")
                    ),
                ));
            }
        }
    }

    /// Collects the `alias` page properties of file pages, in scan order. Runs before
    /// references are resolved, while every page in `by_key` has a file.
    fn build_aliases(&mut self, diags: &mut Vec<Diagnostic>) {
        let mut namespaces = HashSet::new();
        for p in &self.pages {
            for (slash, _) in p.name.match_indices('/') {
                namespaces.insert(NameKey::new(&p.name[..slash]));
            }
        }
        for i in 0..self.pages.len() {
            let page = PageId(i as u32);
            let Some(&first) = self.pages[i].roots.first() else {
                continue;
            };
            let b = self.block(first);
            let Some(value) = b.prop("alias").filter(|_| b.holds_page_props()) else {
                continue;
            };
            let names: Vec<String> = parse::alias_names(value)
                .into_iter()
                .map(normalize_name)
                .collect();
            let file = self.pages[i].file.clone().unwrap_or_default();
            let line = Some(b.line);
            for name in names {
                let key = NameKey::new(&name);
                let problem = if let Err(e) = validate_name(&name) {
                    Some(format!("invalid alias `{name}`: {e}"))
                } else if key == self.pages[i].key {
                    continue;
                } else if Date::parse(&name).is_some() {
                    Some(format!("alias `{name}` is a journal date"))
                } else if let Some(&other) = self.by_key.get(&key) {
                    let other = self.page(other);
                    Some(format!(
                        "alias `{name}` is the name of the page `{}` ({})",
                        other.name,
                        other.file.clone().unwrap_or_default().display()
                    ))
                } else if namespaces.contains(&key) {
                    Some(format!("alias `{name}` is the namespace of other pages"))
                } else {
                    match self.aliases.get(&key) {
                        Some(&other) if other == page => continue,
                        Some(&other) => {
                            let other = self.page(other);
                            Some(format!(
                                "alias `{name}` is already an alias of the page `{}` ({})",
                                other.name,
                                other.file.clone().unwrap_or_default().display()
                            ))
                        }
                        None => None,
                    }
                };
                match problem {
                    Some(message) => diags.push(Diagnostic::new(
                        DiagnosticKind::InvalidAlias,
                        file.clone(),
                        line,
                        format!("{message}; ignored"),
                    )),
                    None => {
                        self.aliases.insert(key, page);
                        self.pages[i].aliases.push(name);
                    }
                }
            }
        }
    }

    fn location(&self, block: BlockId) -> String {
        let b = self.block(block);
        let file = self.page(b.page).file.clone().unwrap_or_default();
        format!("{}:{}", file.display(), b.line)
    }

    fn link_namespaces(&mut self) {
        for i in 0..self.pages.len() {
            let name = &self.pages[i].name;
            if let Some((prefix, _)) = name.rsplit_once('/') {
                let parent = self.by_key[&NameKey::new(prefix)];
                self.pages[i].ns_parent = Some(parent);
                self.pages[parent.0 as usize]
                    .ns_children
                    .push(PageId(i as u32));
            }
        }
        let keys: Vec<NameKey> = self.pages.iter().map(|p| p.key.clone()).collect();
        for page in &mut self.pages {
            page.ns_children
                .sort_by(|a, b| keys[a.0 as usize].cmp(&keys[b.0 as usize]));
        }
    }

    fn build_backlinks(&mut self) {
        let mut backlinks = vec![Vec::new(); self.pages.len()];
        for (id, _) in self.blocks() {
            for page in self.direct_pages(id) {
                backlinks[page.0 as usize].push(id);
            }
        }
        self.backlinks = backlinks;
    }

    fn build_tasks(&mut self) {
        let mut tasks = vec![Vec::new(); self.pages.len()];
        for (id, block) in self.blocks() {
            if block.task.is_none() {
                continue;
            }
            let mut pages = self.inherited_pages(id);
            pages.insert(block.page);
            for page in pages {
                tasks[page.0 as usize].push(id);
            }
        }
        self.tasks = tasks;
    }

    fn build_comentions(&mut self) {
        let mut comentions: HashMap<(PageId, PageId), Vec<BlockId>> = HashMap::new();
        for (id, block) in self.blocks() {
            let mut pages = self.direct_pages(id);
            pages.remove(&block.page);
            let pages: Vec<PageId> = pages.into_iter().collect();
            for (i, &a) in pages.iter().enumerate() {
                for &b in &pages[i + 1..] {
                    comentions.entry((a, b)).or_default().push(id);
                }
            }
        }
        let mut related = vec![Vec::new(); self.pages.len()];
        for &(a, b) in comentions.keys() {
            related[a.0 as usize].push(b);
            related[b.0 as usize].push(a);
        }
        self.comentions = comentions;
        self.related = related;
    }
}

/// Case-insensitive name order with a case-sensitive tie-break, for determinism.
fn display_order(a: &str, b: &str) -> std::cmp::Ordering {
    display_key(a).cmp(&display_key(b))
}

/// Sort key of a page name in [`display_order`].
pub fn display_key(name: &str) -> (String, &str) {
    (name.to_lowercase(), name)
}

struct Loaded {
    file: SourceFile,
    blocks: Vec<ParsedBlock>,
}

fn read_and_parse(root: &Path, files: Vec<SourceFile>, diags: &mut Vec<Diagnostic>) -> Vec<Loaded> {
    let mut loaded = Vec::new();
    for file in files {
        if let Err(e) = validate_name(&file.name) {
            diags.push(Diagnostic::new(
                DiagnosticKind::InvalidPageName,
                file.rel.clone(),
                None,
                format!(
                    "file name gives an invalid page name `{}`: {e}; skipped",
                    file.name
                ),
            ));
            continue;
        }
        let Some(text) = layout::read_source(root, &file.rel, diags) else {
            continue;
        };
        let blocks = parse::parse_blocks(&text);
        loaded.push(Loaded { file, blocks });
    }
    loaded
}

/// `index` is the home page, stored in the root `index.md`: a file under `pages/` that
/// would also be `index` is the page `index (in pages)` instead, which this warns about.
fn warn_pages_index(loaded: &[Loaded], diags: &mut Vec<Diagnostic>) {
    for l in loaded {
        // Its name is already `PAGES_INDEX_PAGE` (see `Address::from_source_path`); only
        // a file named `index` is warned about, not one named `index (in pages)`.
        let named_index = l.file.rel.starts_with(layout::PAGES_DIR)
            && layout::page_name_from_path(&l.file.rel).is_some_and(|n| layout::is_home_name(&n));
        if named_index {
            diags.push(Diagnostic::new(
                DiagnosticKind::ReservedName,
                l.file.rel.clone(),
                None,
                format!(
                    "`index` is the home page (index.md in the notes folder); this file is shown as the page `{}` instead; rename it to avoid confusion",
                    layout::PAGES_INDEX_PAGE
                ),
            ));
        }
    }
}

/// For files mapping to the same page, keeps the one whose path sorts first by bytes.
fn pick_colliding_winners(loaded: &[Loaded], diags: &mut Vec<Diagnostic>) -> Vec<bool> {
    let mut groups: HashMap<NameKey, Vec<usize>> = HashMap::new();
    for (i, l) in loaded.iter().enumerate() {
        groups
            .entry(NameKey::new(&l.file.name))
            .or_default()
            .push(i);
    }
    let mut keep = vec![true; loaded.len()];
    for (i, l) in loaded.iter().enumerate() {
        let group = &groups[&NameKey::new(&l.file.name)];
        if group.len() < 2 || group[0] != i {
            continue;
        }
        let mut members = group.clone();
        members.sort_by(|&a, &b| {
            layout::path_bytes(&loaded[a].file.rel).cmp(layout::path_bytes(&loaded[b].file.rel))
        });
        for &m in &members[1..] {
            keep[m] = false;
        }
        let paths: Vec<String> = members
            .iter()
            .map(|&m| loaded[m].file.rel.display().to_string())
            .collect();
        diags.push(Diagnostic::new(
            DiagnosticKind::CollidingFiles,
            loaded[members[0]].file.rel.clone(),
            None,
            format!(
                "files map to the same page `{}`: {}; using the first",
                l.file.name,
                paths.join(", ")
            ),
        ));
    }
    keep
}

struct Resolver<'d> {
    diags: &'d mut Vec<Diagnostic>,
}

impl Resolver<'_> {
    fn resolve(
        &mut self,
        g: &mut Graph,
        block: BlockId,
        file: &Path,
        raw: RawRef,
        location: RefLocation,
    ) -> Option<Ref> {
        let line = Some(g.block(block).line);
        let target = match raw.kind {
            RawRefKind::Page | RawRefKind::Tag => {
                let name = normalize_name(&raw.text);
                if let Err(e) = validate_name(&name) {
                    self.diags.push(Diagnostic::new(
                        DiagnosticKind::InvalidPageName,
                        file,
                        line,
                        format!("invalid page name `{}`: {e}", raw.text),
                    ));
                    return None;
                }
                // Aliases never name a page with a file, so they come first: an implicit
                // page with the same name exists only as a namespace prefix.
                match g.aliases.get(&NameKey::new(&name)) {
                    Some(&p) => RefTarget::Page(p),
                    None => RefTarget::Page(g.page_for_name(&name)),
                }
            }
            RawRefKind::Block => match g.find_block(&raw.text) {
                Some(b) => RefTarget::Block(b),
                None => {
                    self.diags.push(Diagnostic::new(
                        DiagnosticKind::DanglingBlockRef,
                        file,
                        line,
                        format!("no block has id `{}`", raw.text),
                    ));
                    RefTarget::Unresolved
                }
            },
            RawRefKind::MdLink => match resolve_link_path(file, &raw.text) {
                LinkPath::Outside => {
                    self.diags.push(Diagnostic::new(
                        DiagnosticKind::LinkOutsideRoot,
                        file,
                        line,
                        format!(
                            "link `{}` points outside journals/, pages/, and index.md",
                            raw.text
                        ),
                    ));
                    return None;
                }
                LinkPath::Page(name) => match g
                    .by_key
                    .get(&NameKey::new(&name))
                    .filter(|&&p| !g.page(p).is_implicit())
                {
                    Some(&p) => RefTarget::Page(p),
                    None => {
                        self.diags.push(Diagnostic::new(
                            DiagnosticKind::BrokenLink,
                            file,
                            line,
                            format!("link `{}` points to a missing file", raw.text),
                        ));
                        RefTarget::Unresolved
                    }
                },
            },
        };
        Some(Ref {
            kind: raw.kind,
            text: raw.text,
            range: raw.range,
            dest: raw.dest,
            location,
            target,
        })
    }
}

enum LinkPath {
    /// Name of the page the link's file would store.
    Page(String),
    Outside,
}

/// Resolves a markdown link destination found in `file` to the name of the page stored
/// at that path.
fn resolve_link_path(file: &Path, dest: &str) -> LinkPath {
    let path = dest.split('#').next().unwrap_or_default();
    let Some(path) = percent_decode(path) else {
        // Undecodable destinations cannot name a file; report as missing.
        return LinkPath::Page(String::new());
    };
    let mut parts: Vec<String> = if path.starts_with('/') {
        Vec::new()
    } else {
        file.parent()
            .map(|p| {
                p.components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default()
    };
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return LinkPath::Outside;
                }
            }
            s => parts.push(s.to_owned()),
        }
    }
    let inside = match parts.first().map(String::as_str) {
        Some(PAGES_DIR | JOURNALS_DIR) => parts.len() > 1,
        Some(layout::ROOT_INDEX) => parts.len() == 1,
        _ => false,
    };
    if !inside {
        return LinkPath::Outside;
    }
    // A path inside the notes that is not a page file (`pages/x.png`) is a broken link.
    let rel: PathBuf = parts.iter().collect();
    LinkPath::Page(
        Address::from_source_path(&rel).map_or_else(String::new, |a| a.name().into_owned()),
    )
}
