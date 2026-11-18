"""Preset `ring`: a ring from offsets and a boolean; `half_width` is its slider."""

import bullterrier_vector as btvec

half_width = btvec.Param("half_width", default=14, min=1, max=60, unit="px", meaning="half the ring's width")
disc = btvec.Drawing(btvec.Path.circle((200, 200), 120), size=(400, 400))
drawing = disc.offset(half_width).subtract(disc.offset(-half_width)).fill("#0d0d0d")
