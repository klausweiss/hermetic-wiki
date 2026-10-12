# Spec Delta

## MODIFIED Requirements

### Requirement: Embedded stylesheet and fonts
The output SHALL include hermetic's stylesheet and the Inter and JetBrains Mono fonts under `_hermetic/`, together with the fonts' license texts and the license text of the icons the script uses (`_hermetic/LICENSE-Lucide.txt`), and pages SHALL use only these fonts (Inter for text, JetBrains Mono for code), never fonts installed on the viewer's system except as a last-resort generic fallback.

#### Scenario: Fonts and licenses shipped
- **WHEN** the site is built
- **THEN** `_hermetic/` contains the stylesheet, the Inter and JetBrains Mono font files, their OFL license texts, and the Lucide ISC license text
