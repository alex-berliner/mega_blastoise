"""Minimal ctypes driver for the ngspice shared library (Ubuntu libngspice0, ngspice 42): no CLI needed."""
import ctypes as C, os, tempfile
import numpy as np

LIB = C.CDLL("/usr/lib/x86_64-linux-gnu/libngspice.so.0")
_out = []
SendChar = C.CFUNCTYPE(C.c_int, C.c_char_p, C.c_int, C.c_void_p)
SendStat = C.CFUNCTYPE(C.c_int, C.c_char_p, C.c_int, C.c_void_p)
Exit = C.CFUNCTYPE(C.c_int, C.c_int, C.c_bool, C.c_bool, C.c_int, C.c_void_p)
Data = C.CFUNCTYPE(C.c_int, C.c_void_p, C.c_int, C.c_int, C.c_void_p)
Init = C.CFUNCTYPE(C.c_int, C.c_void_p, C.c_int, C.c_int, C.c_void_p)
BG = C.CFUNCTYPE(C.c_int, C.c_bool, C.c_int, C.c_void_p)
_cb = [SendChar(lambda s, i, p: (_out.append(s.decode(errors="replace")), 0)[1]),
       SendStat(lambda s, i, p: 0), Exit(lambda *a: 0), Data(lambda *a: 0), Init(lambda *a: 0), BG(lambda *a: 0)]
LIB.ngSpice_Init(*_cb, None)

class VecInfo(C.Structure):
    _fields_ = [("v_name", C.c_char_p), ("v_type", C.c_int), ("v_flags", C.c_short),
                ("v_realdata", C.POINTER(C.c_double)), ("v_compdata", C.c_void_p), ("v_length", C.c_int)]
LIB.ngGet_Vec_Info.restype = C.POINTER(VecInfo)
LIB.ngSpice_CurPlot.restype = C.c_char_p

def cmd(s):
    LIB.ngSpice_Command(s.encode())

def run(netlist, analysis):
    """Load netlist text (no .end needed), run an analysis line, return output text."""
    _out.clear()
    cmd("destroy all"); cmd("remcirc")
    lines = [l for l in netlist.strip().splitlines()] + [".end"]
    arr = (C.c_char_p * (len(lines) + 1))(*[l.encode() for l in lines], None)
    LIB.ngSpice_Circ(arr)
    cmd(analysis)
    return "".join(x + "\n" for x in _out)

def vec(name):
    p = LIB.ngGet_Vec_Info(name.encode())
    if not p:
        raise KeyError(name)
    v = p.contents
    if v.v_realdata:
        return np.ctypeslib.as_array(v.v_realdata, shape=(v.v_length,)).copy()
    # complex
    arr = np.ctypeslib.as_array(C.cast(v.v_compdata, C.POINTER(C.c_double)), shape=(2 * v.v_length,)).copy()
    return arr[0::2] + 1j * arr[1::2]

def errors():
    return [l for l in _out if any(k in l.lower() for k in ("error", "fatal", "singular", "abort", "too small"))]
