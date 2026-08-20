"""Minimal reader/writer for KiCad s-expression files.

Quoted atoms are kept distinct from bare ones so a round trip is exact.
"""

class Str(str):
    """An atom that was quoted in the source and must be quoted on the way out."""


def parse(text, i=0):
    i = text.index("(", i)
    node, _ = _parse(text, i)
    return node


def _parse(text, i):
    assert text[i] == "("
    i += 1
    out, cur = [], ""
    while i < len(text):
        c = text[i]
        if c == "(":
            child, i = _parse(text, i)
            out.append(child)
            continue
        if c == ")":
            if cur:
                out.append(cur)
            return out, i + 1
        if c == '"':
            if cur:
                out.append(cur)
                cur = ""
            j, buf = i + 1, ""
            while text[j] != '"':
                if text[j] == "\\":
                    j += 1
                buf += text[j]
                j += 1
            out.append(Str(buf))
            i = j + 1
            continue
        if c in " \t\r\n":
            if cur:
                out.append(cur)
                cur = ""
        else:
            cur += c
        i += 1
    raise ValueError("unbalanced")


def _atom(a):
    if isinstance(a, Str):
        return '"' + a.replace("\\", "\\\\").replace('"', '\\"') + '"'
    return a


def dumps(node, indent=0):
    pad = "\t" * indent
    head = node[0]
    rest = node[1:]
    if not any(isinstance(c, list) for c in rest):
        return pad + "(" + " ".join([_atom(head)] + [_atom(c) for c in rest]) + ")"
    parts = [pad + "(" + _atom(head)]
    inline = []
    for c in rest:
        if isinstance(c, list):
            break
        inline.append(_atom(c))
    if inline:
        parts[0] += " " + " ".join(inline)
    for c in rest[len(inline):]:
        if isinstance(c, list):
            parts.append(dumps(c, indent + 1))
        else:
            parts.append("\t" * (indent + 1) + _atom(c))
    parts.append(pad + ")")
    return "\n".join(parts)
