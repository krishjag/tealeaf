# Canonical test: Timestamps (ISO 8601)
# Tests: date-only, date+time, milliseconds, timezone offsets

# Date only (midnight UTC)
date_only: 2024-01-15

# Date and time (UTC with Z suffix)
datetime_utc: 2024-01-15T10:30:00Z

# With seconds
datetime_seconds: 2024-01-15T10:30:45Z

# With milliseconds
datetime_millis: 2024-01-15T10:30:00.123Z

# With positive timezone offset
datetime_plus_offset: 2024-01-15T10:30:00+05:30

# With negative timezone offset
datetime_minus_offset: 2024-01-15T10:30:00-08:00

# Edge cases
epoch: 1970-01-01T00:00:00Z
y2k: 2000-01-01T00:00:00Z
far_future: 2099-12-31T23:59:59.999Z

# Timestamps in arrays
event_times: [
    2024-01-15T09:00:00Z,
    2024-01-15T12:00:00Z,
    2024-01-15T17:00:00Z,
]

# Timestamps in objects
event: {
    name: "Meeting",
    start: 2024-06-15T14:00:00Z,
    end: 2024-06-15T15:30:00Z,
}

# Timestamps in tables
@struct Event (name: string, scheduled: timestamp)

events: @table Event [
    ("Standup", 2024-01-15T09:00:00Z),
    ("Lunch", 2024-01-15T12:00:00Z),
    ("Review", 2024-01-15T16:00:00Z),
]
