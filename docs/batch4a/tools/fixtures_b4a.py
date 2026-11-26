"""Writes the Batch 4a test projects into a directory (see at_b4a.ps1).
Usage: python fixtures_b4a.py OUTDIR"""
import json, os, sys

out = sys.argv[1]
os.makedirs(out, exist_ok=True)
