#!/usr/bin/env python3
"""Detect drift between the TCGLense API's *response* schemas and this CLI's wire types.

`scripts/check-api-drift.sh` covers the request side of the OpenAPI document —
operations, query parameters and request-body fields. It says nothing about what
comes *back*. Because `--json` re-serialises the typed structs in `src/models.rs`,
a response field a struct does not model is silently dropped from the CLI's output
(that is how `CardDetailResponse`'s printing-level fields, the search `sets` group
and `usd_etched` were missed while the request-side baseline was clean).

This script parses every `pub struct` in `src/models.rs`, pairs each with the
component schema it mirrors, and reports, per struct:

  MISSING      a schema property the struct lacks (dropped from `--json` output)
  STALE        a struct field the schema lacks — `breaks` when the field is
               neither `Option` nor `#[serde(default)]`, since decoding then fails
  NULLABILITY  a nullable / not-required property modelled as a bare non-`Option`
               field with no default — decoding fails as soon as it is null/absent
  TYPE         a field whose Rust type cannot hold what the schema sends
  REF          a nested struct that maps to a different schema than the property's

plus, across the spec:

  UNMODELLED   an object schema some 2xx response carries (directly or via a
               `Page_` / `DataBody_` / `SearchGroup_` wrapper) that no struct maps to
  UNMAPPED     a struct no schema could be paired with (a rename, or a type the
               API stopped documenting) — see NO_SCHEMA for the known exceptions

A struct pairs with the schema of the same name, else `<Name>Response`, else an
entry in ALIASES. Generic wrappers (`Page<T>`, `DataBody<T>`, `SearchGroup<T>`)
are skipped: the spec inlines their item schema per instantiation.

Exit status: 0 when nothing drifts, 1 on drift (so it can gate CI like the
request-side check), 2 on a usage/fetch error.

Usage:
  scripts/check-schema-drift.py                       # live prod spec
  scripts/check-schema-drift.py http://localhost:5173  # a local/self-host API
  scripts/check-schema-drift.py path/to/openapi.json   # an already-downloaded spec
  scripts/check-schema-drift.py --models src/models.rs # override the wire-types file

Requires: Python 3.8+, standard library only.
"""

import argparse
import json
import os
import re
import sys
import urllib.error
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
DEFAULT_MODELS = os.path.join(HERE, "..", "src", "models.rs")

# Structs whose schema name is neither `<Name>` nor `<Name>Response`.
ALIASES = {
    "ArtTag": "ArtTagEntry",
    "Keyword": "KeywordEntry",
    "CollectionAdd": "CollectionAddSummary",
    "NeededList": "NeededCards",
    "Releases": "ReleaseCalendarResponse",
    "CardPrices": "PricesResponse",
    "ProductPrices": "ProductPricesResponse",
    "CardSet": "SetResponse",
    "CollectionBreakdown": "HoldingBreakdown",
    "ProductOpening": "PackOpening",
    "LifePlayer": "LifeSeatResponse",
    "IngestStatus": "StatusResponse",
}

# Structs the spec documents no schema for, and why. Anything else that fails to
# pair is reported as UNMAPPED so a rename upstream shows up.
NO_SCHEMA = {
    "User": "auth routes are undocumented in the spec",
    "AuthResponse": "auth routes are undocumented in the spec",
    "RegisterResponse": "auth routes are undocumented in the spec",
    "PublicConfig": "served outside the documented API surface",
    "CurrencyRatesResponse": "served outside the documented API surface",
    "UsernameAvailability": "auth routes are undocumented in the spec",
    "CollectionSet": "the collection sets route's DataBody carries an unresolved item ref",
}

# Response schemas the CLI decodes through a generic wrapper instead of a struct
# of their own, and the wrapper it uses. Their `data` item is checked like any
# other nested ref, so growth inside the item still surfaces on the item's struct.
WRAPPED = {
    "ScanResponse": "DataBody<Vec<ScanMatch>>",
    "DataBody": "the bare generic wrapper: the spec leaves its item ref unresolved (collection sets)",
}

WRAPPER_PREFIXES = ("Page_", "DataBody_", "SearchGroup_")
RUST_INTS = {"i8", "i16", "i32", "i64", "isize", "u8", "u16", "u32", "u64", "usize"}


# ---------------------------------------------------------------------------
# Rust side
# ---------------------------------------------------------------------------


def parse_structs(src):
    """Every `pub struct` in the file → {name: {"generic": bool, "fields": {wire_name: meta}}}.

    `meta` is {"ty": rust type, "default": bool, "flatten": bool, "rust_name": str}.
    Serde attributes are read from the `#[serde(...)]` lines directly above a field.
    """
    structs = {}
    pattern = re.compile(r"pub struct (\w+)(<[^>]*>)?\s*\{(.*?)\n\}", re.S)
    for m in pattern.finditer(src):
        name, generic, body = m.group(1), m.group(2), m.group(3)
        fields = {}
        pending = {}
        for raw in body.split("\n"):
            line = raw.strip()
            if line.startswith("#[serde"):
                if re.search(r"\bdefault\b", line):
                    pending["default"] = True
                if re.search(r"\bflatten\b", line):
                    pending["flatten"] = True
                if re.search(r"\bskip\b", line):
                    pending["skip"] = True
                r = re.search(r'rename\s*=\s*"([^"]+)"', line)
                if r:
                    pending["rename"] = r.group(1)
                continue
            if line.startswith("#[") or line.startswith("//"):
                continue
            fm = re.match(r"pub (?:r#)?(\w+): (.+?),?$", line)
            if fm:
                if not pending.get("skip"):
                    wire = pending.get("rename", fm.group(1))
                    fields[wire] = {
                        "ty": fm.group(2),
                        "default": pending.get("default", False),
                        "flatten": pending.get("flatten", False),
                        "rust_name": fm.group(1),
                    }
                pending = {}
        structs[name] = {"generic": generic is not None, "fields": fields}
    return structs


def rust_kind(ty):
    """A Rust type → (kind, nullable, inner) where kind is a JSON-ish category
    (string / integer / number / boolean / array / object / ref / any) and inner
    is the element type for arrays or the struct name for refs."""
    nullable = False
    t = ty.strip()
    while True:
        m = re.match(r"^(?:Option|Box)<(.+)>$", t)
        if not m:
            break
        if t.startswith("Option"):
            nullable = True
        t = m.group(1).strip()
    m = re.match(r"^Vec<(.+)>$", t)
    if m:
        return "array", nullable, m.group(1).strip()
    if t == "String" or t == "&str":
        return "string", nullable, None
    if t in RUST_INTS:
        return "integer", nullable, None
    if t in ("f32", "f64"):
        return "number", nullable, None
    if t == "bool":
        return "boolean", nullable, None
    if re.match(r"^(BTreeMap|HashMap)<", t):
        return "object", nullable, None
    if "Value" in t:
        return "any", nullable, None
    return "ref", nullable, t


# ---------------------------------------------------------------------------
# Spec side
# ---------------------------------------------------------------------------


class Spec:
    def __init__(self, doc):
        self.doc = doc
        self.schemas = doc.get("components", {}).get("schemas", {})

    def ref_name(self, node):
        ref = node.get("$ref")
        return ref.rsplit("/", 1)[-1] if ref else None

    def resolve(self, node):
        """Follow `$ref` (and a single-member `allOf`) to the schema it names."""
        seen = 0
        while seen < 20:
            seen += 1
            if "$ref" in node:
                name = self.ref_name(node)
                if name not in self.schemas:
                    return {"type": "object", "x-dangling": name}
                node = self.schemas[name]
            elif "allOf" in node and len(node["allOf"]) == 1:
                node = node["allOf"][0]
            else:
                return node
        return node

    def object_shape(self, schema):
        """(properties, required) of an object schema, merging `allOf` parts."""
        props, req = {}, set()
        parts = schema["allOf"] if "allOf" in schema else [schema]
        for part in parts:
            part = self.resolve(part)
            props.update(part.get("properties", {}))
            req |= set(part.get("required", []))
        return props, req

    def kind(self, prop):
        """A property schema → (kind, nullable, ref_name_or_None, items_schema_or_None)."""
        nullable = False
        p = prop
        if "allOf" in p and len(p["allOf"]) == 1:
            p = p["allOf"][0]
        if "oneOf" in p or "anyOf" in p:
            alts = p.get("oneOf") or p.get("anyOf")
            real = [a for a in alts if a.get("type") != "null"]
            nullable = len(real) < len(alts)
            if len(real) == 1:
                p = real[0]
            else:
                return "oneOf", nullable, None, None
        if "$ref" in p:
            name = self.ref_name(p)
            target = self.resolve(p)
            if target.get("enum"):
                return "string", nullable, name, None
            t = target.get("type", "object")
            if isinstance(t, list):
                if "null" in t:
                    nullable = True
                t = [x for x in t if x != "null"]
                t = t[0] if t else "any"
            return t, nullable, name, target.get("items")
        t = p.get("type")
        if isinstance(t, list):
            if "null" in t:
                nullable = True
            t = [x for x in t if x != "null"]
            t = t[0] if t else "any"
        if t is None:
            t = "object" if "properties" in p else "any"
        return t, nullable, None, p.get("items")

    def response_schemas(self):
        """Names of object schemas any 2xx response carries, directly or through a
        `Page_` / `DataBody_` / `SearchGroup_` wrapper, following `$ref`s inside."""
        roots = set()
        for path in self.doc.get("paths", {}).values():
            for op in path.values():
                if not isinstance(op, dict):
                    continue
                for code, resp in op.get("responses", {}).items():
                    if not str(code).startswith("2"):
                        continue
                    for media in resp.get("content", {}).values():
                        name = self.ref_name(media.get("schema", {}))
                        if name:
                            roots.add(name)
        reachable = set()
        stack = list(roots)
        while stack:
            name = stack.pop()
            if name in reachable:
                continue
            if name.startswith(WRAPPER_PREFIXES):
                # `Page_CardResponse` inlines its item; recover the item name.
                inner = name.split("_", 1)[1]
                for pfx in ("Vec_", "Option_"):
                    if inner.startswith(pfx):
                        inner = inner[len(pfx):]
                if inner in self.schemas:
                    stack.append(inner)
                schema = self.schemas.get(name, {})
            else:
                if name not in self.schemas:
                    continue
                reachable.add(name)
                schema = self.schemas[name]
            stack.extend(self.refs_in(schema))
        return {
            n
            for n in reachable
            if not self.schemas[n].get("enum")
            and self.resolve(self.schemas[n]).get("type", "object") not in ("string", "integer", "number", "boolean")
        }

    def refs_in(self, node):
        found = []
        if isinstance(node, dict):
            if "$ref" in node:
                found.append(self.ref_name(node))
            for v in node.values():
                found.extend(self.refs_in(v))
        elif isinstance(node, list):
            for v in node:
                found.extend(self.refs_in(v))
        return found


# ---------------------------------------------------------------------------
# Comparison
# ---------------------------------------------------------------------------


def schema_for(name, spec):
    for cand in (name, name + "Response", ALIASES.get(name)):
        if cand and cand in spec.schemas:
            return cand
    return None


def ref_matches(rust_ty, spec_ref, spec):
    """Whether a nested Rust type mirrors the schema a property refs. A generic
    instantiation (`SearchGroup<Card>`) matches the spec's mangled name for it
    (`SearchGroup_CardResponse`), item struct checked through the same pairing."""
    m = re.match(r"^(\w+)<(.+)>$", rust_ty)
    if m:
        wrapper, inner = m.group(1), m.group(2).strip()
        if not spec_ref.startswith(wrapper + "_"):
            return False
        want = spec_ref[len(wrapper) + 1:]
        for pfx in ("Vec_", "Option_"):
            if want.startswith(pfx):
                want = want[len(pfx):]
        ik, _, iinner = rust_kind(inner)
        if ik == "ref":
            return schema_for(iinner, spec) == want
        return True
    return schema_for(rust_ty, spec) == spec_ref


def compatible(spec_kind, rust_kind_, spec_prop):
    """Whether a Rust kind can hold a spec kind (lenient where serde is lenient)."""
    if spec_kind == rust_kind_ or rust_kind_ == "any" or spec_kind in ("any", "oneOf"):
        return True
    if spec_kind == "object" and rust_kind_ == "ref":
        return True
    if spec_kind == "string" and rust_kind_ == "ref":
        return True  # a String-backed enum newtype
    if spec_kind == "integer" and rust_kind_ == "number":
        return True  # f64 decodes an integer
    if spec_kind == "number" and rust_kind_ == "integer":
        return str(spec_prop.get("format", "")).startswith("int")
    return False


def effective_fields(name, structs):
    """A struct's fields with any `#[serde(flatten)]` inner struct merged in."""
    own = structs[name]["fields"]
    merged = {}
    for wire, meta in own.items():
        if meta["flatten"]:
            _, _, inner = rust_kind(meta["ty"])
            if inner in structs:
                merged.update(effective_fields(inner, structs))
            continue
        merged[wire] = meta
    return merged


def check_struct(name, structs, spec):
    """Findings for one struct as a list of (code, message)."""
    schema_name = schema_for(name, spec)
    if not schema_name:
        return None, [("UNMAPPED", f"{name}: no schema named {name}, {name}Response or in ALIASES")]
    schema = spec.schemas[schema_name]
    props, required = spec.object_shape(schema)
    fields = effective_fields(name, structs)
    out = []

    for wire, meta in fields.items():
        rk, rn, inner = rust_kind(meta["ty"])
        tolerant = rn or meta["default"]
        if wire not in props:
            sev = "ok" if tolerant else "breaks"
            out.append(("STALE", f"{name}.{wire}: `{meta['ty']}` is not in {schema_name} ({sev})"))
            continue
        prop = props[wire]
        sk, sn, sref, items = spec.kind(prop)
        if (sn or wire not in required) and not tolerant:
            why = "nullable" if sn else "not required"
            out.append(("NULLABILITY", f"{name}.{wire}: `{meta['ty']}` but {schema_name}.{wire} is {why}"))
        if not compatible(sk, rk, prop):
            out.append(("TYPE", f"{name}.{wire}: `{meta['ty']}` cannot hold {schema_name}.{wire} ({sk})"))
            continue
        if rk == "ref" and sref and sref in spec.schemas and not spec.schemas[sref].get("enum"):
            if not ref_matches(inner, sref, spec):
                out.append(("REF", f"{name}.{wire}: `{inner}` does not mirror {sref} ({schema_name}.{wire})"))
        if rk == "array" and items is not None:
            ik, _, iref, _ = spec.kind(items)
            ek, _, einner = rust_kind(inner)
            if not compatible(ik, ek, items):
                out.append(("TYPE", f"{name}.{wire}: `Vec<{inner}>` cannot hold {schema_name}.{wire} items ({ik})"))
            elif ek == "ref" and iref and iref in spec.schemas and not spec.schemas[iref].get("enum"):
                if not ref_matches(einner, iref, spec):
                    out.append(("REF", f"{name}.{wire}: `Vec<{einner}>` does not mirror {iref} ({schema_name}.{wire} items)"))

    for wire, prop in props.items():
        if wire in fields:
            continue
        sk, sn, sref, _ = spec.kind(prop)
        shape = sk + ("?" if sn else "")
        if sref:
            shape += f" ({sref})"
        opt = "" if wire in required else ", optional"
        out.append(("MISSING", f"{name}.{wire}: {schema_name} has `{wire}`: {shape}{opt}"))
    return schema_name, out


def run(spec, structs):
    findings = []
    covered = set()
    for name, info in structs.items():
        if info["generic"]:
            continue
        if name in NO_SCHEMA:
            continue
        schema_name, out = check_struct(name, structs, spec)
        if schema_name:
            covered.add(schema_name)
        findings.extend(out)
    # A schema a covered one extends via `allOf` (ComboSummary under DeckCombo) is
    # checked through the extending struct, so it counts as covered too.
    for name in list(covered):
        for part in spec.schemas.get(name, {}).get("allOf", []):
            ref = spec.ref_name(part)
            if ref:
                covered.add(ref)
    for name in sorted(spec.response_schemas()):
        if name in covered or name in WRAPPED:
            continue
        findings.append(("UNMODELLED", f"{name}: a 2xx response carries it but no struct in models.rs maps to it"))
    return findings


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def load_spec(source):
    if os.path.exists(source):
        with open(source, encoding="utf-8") as fh:
            return json.load(fh), source
    url = source.rstrip("/") + "/api/openapi.json"
    # The site's edge rejects Python's default user agent with a 403.
    req = urllib.request.Request(url, headers={"User-Agent": "tcglense-cli check-schema-drift"})
    try:
        with urllib.request.urlopen(req, timeout=45) as resp:
            return json.load(resp), url
    except (urllib.error.URLError, ValueError) as e:
        print(f"error: could not fetch {url}: {e}", file=sys.stderr)
        sys.exit(2)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0], add_help=True)
    ap.add_argument("source", nargs="?", default="https://tcglense.com",
                    help="API origin (default https://tcglense.com) or a path to an openapi.json")
    ap.add_argument("--models", default=DEFAULT_MODELS, help="wire-types file (default src/models.rs)")
    args = ap.parse_args()

    doc, where = load_spec(args.source)
    spec = Spec(doc)
    if not spec.schemas:
        print(f"error: {where} has no components.schemas", file=sys.stderr)
        sys.exit(2)
    try:
        with open(args.models, encoding="utf-8") as fh:
            structs = parse_structs(fh.read())
    except OSError as e:
        print(f"error: {e}", file=sys.stderr)
        sys.exit(2)
    if not structs:
        print(f"error: no `pub struct` found in {args.models}", file=sys.stderr)
        sys.exit(2)

    version = doc.get("info", {}).get("version", "?")
    checked = sum(1 for n, i in structs.items() if not i["generic"] and n not in NO_SCHEMA)
    print(f"TCGLense API {where} (v{version})")
    print(f"models: {len(structs)} structs ({checked} checked against {len(spec.schemas)} schemas)")

    findings = run(spec, structs)
    if not findings:
        print()
        print("No drift: every checked struct matches its response schema field for field.")
        return 0

    order = ["NULLABILITY", "TYPE", "REF", "STALE", "MISSING", "UNMODELLED", "UNMAPPED"]
    blurb = {
        "NULLABILITY": "Fields that will FAIL TO DECODE when the API sends null / omits them (make them Option or #[serde(default)])",
        "TYPE": "Fields whose Rust type cannot hold what the schema sends",
        "REF": "Nested structs paired with a different schema than the property names",
        "STALE": "Struct fields the schema no longer has (`breaks` = decoding fails once the API drops it)",
        "MISSING": "Schema properties the struct lacks — dropped from `--json` output (add the field, and render it)",
        "UNMODELLED": "Response schemas no struct models — a route payload the CLI never decodes fully",
        "UNMAPPED": "Structs no schema could be paired with (renamed upstream? add to ALIASES or NO_SCHEMA)",
    }
    for code in order:
        rows = [msg for c, msg in findings if c == code]
        if not rows:
            continue
        print()
        print(f"== {code}: {blurb[code]} ==")
        for msg in rows:
            print(f"  {msg}")
    print()
    print(f"Drift detected: {len(findings)} finding(s).")
    return 1


if __name__ == "__main__":
    sys.exit(main())
