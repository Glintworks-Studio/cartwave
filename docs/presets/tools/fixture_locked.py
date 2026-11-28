"""A locked project without lime in its colours (the lime scenes): fx_studio (fixtures_b4b.py)
under the NES profile, Lead locked to Pulse 1, plus a locked DPCM track for the Sampler.
usage: python fixture_locked.py <fx_studio.chipdaw> <out.chipdaw>"""
import json
import sys

v = json.load(open(sys.argv[1], encoding="utf-8"))
v["profile"] = "nes"
v["tracks"][0]["lock"] = "2a03_p1"
t = dict(v["tracks"][1])
t.update(id=3, name="Drums", voice="dpcm", color="#E0A060", lock="2a03_dpcm")
v["tracks"].append(t)
json.dump(v, open(sys.argv[2], "w", encoding="utf-8"), indent=1)
