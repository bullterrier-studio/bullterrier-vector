"""Cut paper, the second reference piece: a dancer traced from a mask into stacked paper layers.

A segmentation mask per frame (from the caller) becomes a black cut-out on stacked paper: a fat, blobby mount with a dot
screen, two echoes the size of the figure shifted in opposite directions, a drop shadow under every layer, a word pasted
underneath, and an outline that draws itself on around the figure on each beat. Paper slides; it never fades.

Masks are plain data (width, height, bytes). Each frame is traced together with its neighbouring masks, so the edge
stays steady and holes do not flicker; the library keeps no sequence itself.
"""

from pathlib import Path

import bullterrier_vector as btvec

WIDTH, HEIGHT = 1080, 1350
MASK_WIDTH, MASK_HEIGHT = 540, 675
FPS = 25  # the masks' frame rate
BEAT = 60 / 112  # seconds per beat
PAPER, MOUNT, DOTS, ECHO_A, ECHO_B, CUTOUT = "#efe6d6", "#f2c14e", "#d1495b", "#e4572e", "#2e86ab", "#121212"

# One 8-bit grayscale map per frame, the dancer white, stretched over the frame.
masks = [
    btvec.Field.map(MASK_WIDTH, MASK_HEIGHT, file.read_bytes(), box=(0, 0, WIDTH, HEIGHT))
    for file in sorted(Path("masks").glob("*.gray"))
]
font = btvec.Font(Path("fonts/Anybody[wdth,wght].ttf").read_bytes())

slide = btvec.Param("slide", default=1, min=0, max=1.2, unit="fraction", meaning="how far the echoes have slid out")
outline = btvec.Param("outline", default=1, min=0, max=1, unit="fraction", meaning="how much outline has drawn on")

word = btvec.text(
    "MOVE", font=font, size=420, axes={"wght": 900}, at=(WIDTH / 2, 1250), align="center", width=1000, fit="wdth"
)


def paper(layer: btvec.Drawing, color: str) -> btvec.Drawing:
    """A layer of flat paper lifted by a drop shadow."""
    return layer.fill(color).shadow(dy=10, blur=18, color="#0000004d")


def frame_at(i: int) -> btvec.Drawing:
    """The paper stack for mask `i`, traced with up to two masks on each side."""
    figure = (
        btvec.trace(masks[i], levels=[0.5], before=masks[max(0, i - 2) : i], after=masks[i + 1 : i + 3])
        .offset(-6)
        .offset(6)  # shrink, then grow back: shaves off spurs thinner than 12 px
        .drop_parts(smaller_than=1500)
        .fill_holes(smaller_than=4000)
    )
    mount = figure.offset(110).offset(-70)  # grow by 40 px, closing gaps and dents up to 70 px: one fat blob
    echo = figure.offset(16)
    return btvec.Drawing(
        btvec.Path.rect(0, 0, WIDTH, HEIGHT).fill(PAPER),
        word.fill(CUTOUT),
        paper(mount, MOUNT),
        mount.halftone(DOTS, tone=0.7, pitch=14, angle=45),
        paper(echo.translate(-70 * slide, 24 * slide), ECHO_A),
        paper(echo.translate(70 * slide, -24 * slide), ECHO_B),
        paper(figure, CUTOUT),
        figure.offset(26).slice(0, outline).stroke(CUTOUT, width=5),
        size=(WIDTH, HEIGHT),
    )


def at_time(t: float) -> btvec.Drawing:
    """The piece `t` seconds in: on each beat the echoes tuck in and slide out, and the outline draws on again."""
    since_beat = t % BEAT
    return frame_at(round(t * FPS)).with_values(
        slide=btvec.ease.out_back(since_beat / 0.3),
        outline=btvec.ease.out_cubic(since_beat / (0.8 * BEAT)),
    )


Path("cut-paper.png").write_bytes(at_time(4.0).to_png(WIDTH, HEIGHT))
btvec.frames(at_time, width=WIDTH, height=HEIGHT, fps=FPS, seconds=len(masks) / FPS, out="cut-paper.mov")
