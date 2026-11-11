"""A logo imported from SVG, drawn part by part: the SVG's own ids name the parts, so `only(id=...)` picks one.

`Drawing.from_svg` keeps each element's `id`, `class` and `group` (the ids of the groups round it, joined by `/`).
`class` is a Python keyword, so it is selected through a dict, and it matches the whole attribute text.
"""

import bullterrier_vector as btvec

LOGO = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 400">
  <g id="logo">
    <circle id="ring" cx="200" cy="200" r="150" fill="none" stroke="#1a1a1a" stroke-width="24"/>
    <g id="mark">
      <path id="spark" fill="#ffcc33" d="M200 90 L230 190 L330 200 L230 210 L200 310 L170 210 L70 200 L170 190 Z"/>
      <rect id="dot" class="accent" x="190" y="40" width="20" height="20" fill="#e63946"/>
    </g>
  </g>
</svg>"""

logo = btvec.Drawing.from_svg(LOGO)
ring = logo.only(id="ring")  # one part by its id
mark = logo.only(group="logo/mark")  # the parts inside the group `mark` of the group `logo`
accent = logo.only(**{"class": "accent"})  # by class: the whole text of the attribute
shapes = ring.shapes()  # the ring alone: its stroke is 24 wide, no fill
svg = ring.to_svg(400, 400)
