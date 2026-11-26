"""A pure-Python stand-in for the few Pillow calls the Windows passes' checkers use
(lime_audit*.py, banding.py, mockup_match.py), so they run unchanged on this Mac without
installing anything (MAC_CATCHUP §0). 8-bit RGB / RGBA / palette / grey PNGs, no interlace.
Use: PYTHONPATH=docs/mac-catchup/tools/pilshim python3 <tool>.py ..."""
