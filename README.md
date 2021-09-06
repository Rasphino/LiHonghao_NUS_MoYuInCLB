# Robot rate calculator

## How to run?
```
cargo run
```

Then input in json format: 
```json
{
  "shift": {
    "start": "2038-01-01T20:15:00",
    "end": "2038-01-02T04:15:00"
  },
  "roboRate": {
    "standardDay": {
      "start": "07:00:00",
      "end": "23:00:00",
      "value": 20
    },
    "standardNight": {
      "start": "23:00:00",
      "end": "07:00:00",
      "value": 25
    },
    "extraDay": {
      "start": "07:00:00",
      "end": "23:00:00",
      "value": 30
    },
    "extraNight": {
      "start": "23:00:00",
      "end": "07:00:00",
      "value": 35
    }
  }
}
```

Or use input redirect:
```
cargo run < sample_input.json
```

This calculator will produce the result in json format, for example:
```
{"value":13725}
```


## Problem assumptions
1. There are no gaps between day and night, i.e., if the day ends at 23:00, then the night must start at 23:00.
2. Only four kinds of rates are supported, but the library part can be easily modified to support more rate schemes.

## Todos
1. Refactor the library part to support rate scheme independent counting.
