# ltsv

[Labeled Tab-separated Values](http://ltsv.org/) parser in Rust and no-std compatible.

```rust
use ltsv::{tokenize, Pair};

let data_iterator = tokenize("mylabel:1234\tmore_data:text");

for line in data_iterator {
    for possible_pair in line {
        if let Ok(pair) = possible_pair {
            let data = Pair::from(pair);
            // do something with the data
            let label = data.label;
            let field = data.field;
        }
    }
}
```

The [`Data`] and [`Record`] structs both implement the Iterator behaviour, which allows to extract the data lazily.

## Features

### std

If the std feature is enabled there are some extra helper functions you can use.
For instance the [`parse`] function which early extracts all the data and puts it in a `Vec`

```rust
use ltsv::{parse, Pair};

let out = parse("mylabel:1234\tmore_data:text");
let lines = out.unwrap();

assert_eq!(Pair{label: "mylabel", field: "1234"}, lines[0][0]);
```

## More examples

print results back to ltsv:

```rust
use ltsv::Pair;

let data = [Pair{label: "my_label", field: "testing"}, Pair{label: "my_label2", field: "testing"}];
let out = data.map(|x| x.to_string()).join(&ltsv::TAB.to_string());

assert_eq!("my_label:testing\tmy_label2:testing", out);
```

Side note: This is not unicode aware, but I followed the original grammar when implement this
