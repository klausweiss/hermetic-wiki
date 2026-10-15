# hermetic's icons, drawn as pixel art by Python scripts (see lid.py). The icons
# under static/ are committed, so building hermetic needs none of this; rerun
# after changing a script. Needs python3 and rsvg-convert (librsvg).

.PHONY: all icons art clean

STATIC := ../../static
OUT := out

all: icons art

icons: $(STATIC)/favicon.svg $(STATIC)/apple-touch-icon.png

# Browser tabs: a 32x32 grid.
$(STATIC)/favicon.svg: favicon.py lid.py
	python3 favicon.py $@

# Bookmark tiles and home screens: 180x180, opaque (iOS shows transparency as
# black), the detailed lid on a 90x90 grid drawn at exactly 2x.
$(STATIC)/apple-touch-icon.png: detailed.py lid.py | $(OUT)
	python3 detailed.py $(OUT)/apple-touch-icon.svg --size 74 --pad 8 --background '#fbfbfa'
	rsvg-convert -w 180 -h 180 $(OUT)/apple-touch-icon.svg -o $@

# The detailed lid on its own: a 128x128 grid at 1024x1024.
art: $(OUT)/lid.png

$(OUT)/lid.png: detailed.py lid.py | $(OUT)
	python3 detailed.py $(OUT)/lid.svg
	rsvg-convert -w 1024 -h 1024 $(OUT)/lid.svg -o $@

$(OUT):
	mkdir -p $@

clean:
	rm -rf $(OUT) __pycache__
