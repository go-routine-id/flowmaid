import re, sys, math, os
sys.path.insert(0, os.path.dirname(__file__))
# pinover.py <svgdir> [thr] [minrun]: list (file, pin, other, run px) where a frame pin runs within thr px of another edge
from near import parse  # noqa
