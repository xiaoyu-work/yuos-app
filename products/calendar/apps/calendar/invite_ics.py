"""Read a single, non-recurring iCalendar invitation or cancellation."""

import re


MAX_ICS_BYTES = 64 * 1024
_CONTENT_LINE = re.compile(r'([A-Z0-9-]+)(;(?:(?:[^":])|"[^"]*")*)?:(.*)', re.ASCII | re.IGNORECASE)
_UTC_DATE_TIME = re.compile(r"([0-9]{4})([0-9]{2})([0-9]{2})T([0-9]{2})([0-9]{2})([0-9]{2})Z")
_MAILBOX = re.compile(r"[A-Za-z0-9.!$&'*+\-/=_~]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*")
_UNSUPPORTED = {"RRULE", "RDATE", "EXRULE", "EXDATE", "RECURRENCE-ID", "DURATION"}


def _properties(lines, parameters, *, event=False):
    found = {}
    for line in lines:
        if not (match := _CONTENT_LINE.fullmatch(line)):
            raise ValueError("invalid iCalendar content line")
        name, params, value = match.groups()
        name = name.upper()
        if name in {"BEGIN", "END"} or (event and name in _UNSUPPORTED):
            raise ValueError("unsupported iCalendar component or event property")
        if name not in parameters:
            continue
        if name == "ATTENDEE":
            found.setdefault(name, []).append(value)
            continue
        if name in found or not re.fullmatch(parameters[name], params or "", re.ASCII | re.IGNORECASE):
            raise ValueError(f"duplicate or unsupported {name} property")
        found[name] = value
    return found


def _text(value):
    def unescape(match):
        escaped = match[1].lower()
        if escaped == "n":
            return "\n"
        if escaped not in {"\\", ";", ","}:
            raise ValueError("invalid iCalendar text escape")
        return escaped

    return re.sub(r"\\(.|$)", unescape, value)


def _timestamp(value):
    if not (match := _UTC_DATE_TIME.fullmatch(value)):
        raise ValueError("invitation times must be UTC DATE-TIME values")
    return "{}-{}-{}T{}:{}:{}Z".format(*match.groups())


def normalize_mailbox(address):
    if not isinstance(address, str) or len(address) > 254 or not _MAILBOX.fullmatch(address):
        raise ValueError("expected a plain invitation email address")
    local, domain = address.rsplit("@", 1)
    return f"{local}@{domain.lower()}"


def _mailbox_uri(value):
    if value[:7].lower() != "mailto:":
        raise ValueError("invitation participants must use mailto addresses")
    return normalize_mailbox(value[7:])


def parse_invitation(icalendar):
    """Return the identity, sequence, attendees and fields (None for a cancellation)."""
    if not isinstance(icalendar, str) or len(icalendar) > MAX_ICS_BYTES:
        raise ValueError("expected an iCalendar string of at most 64 KiB")
    try:
        size = len(icalendar.encode("utf-8"))
    except UnicodeError:
        raise ValueError("invalid iCalendar text") from None
    if size > MAX_ICS_BYTES:
        raise ValueError("expected an iCalendar string of at most 64 KiB")
    text = icalendar.replace("\r\n", "\n")
    if re.search(r"[\x00-\x08\x0b-\x1f\x7f-\x9f]", text):
        raise ValueError("invalid control character in iCalendar")
    lines = re.sub(r"\n[ \t]", "", text).removesuffix("\n").split("\n")
    envelope = [line.upper() for line in lines]
    if ("BEGIN:VEVENT" not in envelope or envelope[:1] != ["BEGIN:VCALENDAR"]
            or envelope[-2:] != ["END:VEVENT", "END:VCALENDAR"]):
        raise ValueError("expected a VCALENDAR with one VEVENT")
    begin = envelope.index("BEGIN:VEVENT")
    header = _properties(lines[1:begin], {"VERSION": "", "METHOD": ""})
    method = header.get("METHOD", "").upper()
    if header.get("VERSION") != "2.0" or method not in {"REQUEST", "CANCEL"}:
        raise ValueError("expected an iCalendar 2.0 REQUEST or CANCEL")
    language = r"(?:;LANGUAGE=[A-Z0-9-]+)?"
    datetime_params = r"(?:;VALUE=DATE-TIME)?"
    props = _properties(lines[begin + 1:-2], {
        "UID": "", "SEQUENCE": "", "STATUS": "",
        "ORGANIZER": ".*", "ATTENDEE": ".*",
        "SUMMARY": language, "LOCATION": language, "DESCRIPTION": language,
        "DTSTART": datetime_params, "DTEND": datetime_params,
    }, event=True)
    uid = _text(props.get("UID", ""))
    if not uid or uid != uid.strip() or "\n" in uid or "\t" in uid or len(uid.encode()) > 255:
        raise ValueError("invalid invitation UID")
    raw_sequence = props.get("SEQUENCE", "0")
    if not re.fullmatch(r"[0-9]{1,10}", raw_sequence) or (sequence := int(raw_sequence)) > 2147483647:
        raise ValueError("invalid invitation SEQUENCE")
    organizer = _mailbox_uri(props.get("ORGANIZER", ""))
    attendees = sorted({_mailbox_uri(value) for value in props.get("ATTENDEE", [])})
    if method == "CANCEL":
        if "SEQUENCE" not in props:
            raise ValueError("cancellation requires a SEQUENCE")
        return uid, sequence, organizer, attendees, None
    title = _text(props.get("SUMMARY", ""))
    if not title.strip() or props.get("STATUS", "").upper() == "CANCELLED":
        raise ValueError("expected a titled, non-cancelled invitation")
    if not attendees:
        raise ValueError("invitation requires an attendee")
    return uid, sequence, organizer, attendees, {
        "title": title,
        "start": _timestamp(props.get("DTSTART", "")),
        "end": _timestamp(props.get("DTEND", "")),
        "description": _text(props.get("DESCRIPTION", "")),
        "location": _text(props.get("LOCATION", "")),
    }
