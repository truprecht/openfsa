# openfsa

This library crate is a wrapper around a small part of [openfst](http://www.openfst.org/) that offers basic functions to work with *finite state automata*.

The installation requires the shared libraries and headers of *openfst* installed in the search path of your compiler.
Please follow the instructions listed [here](http://www.openfst.org/twiki/bin/view/FST/FstDownload) to download and install *openfst*.
Keep in mind that *openfst* itself is licensed using the [Apache license v2](http://www.apache.org/licenses/LICENSE-2.0).

## Usage

We can construct an `Automaton` using an initial state, a list of final states and a list of `Arc`s.
```rust
use openfsa::fsa::{Arc, Automaton};
let arcs: Vec<Arc<&str, &str>>
    = vec![
        Arc {
            from: "q1",
            to: "q2",
            label: "a",
            weight: LogDomain::new(0.9).unwrap(),
        },
        Arc {
            from: "q2",
            to: "q1",
            label: "word",
            weight: LogDomain::one(),
        },
    ];
let fsa: Automaton<&str> = Automaton::from_arcs("q", vec!["q"], arcs);
```

We can intersect Automata,
```rust
let intersection = fsa.intersect(&fsa);
```
remove the language recognized by a second fsa from the first one,
```rust
let nothing = intersection.difference(&fsa);
```
dump automata to, and read them from binary strings using the [serde](https://github.com/serde-rs/serde) framework,
```rust
extern crate serde_json;
println!("{}", serde_json::to_string(&intersection).unwrap());
```
and finally, we can also enumerate all words recognized by an automaton.
```rust
use openfsa::fsa::generator::BatchGenerator;
for word in BatchGenerator::new(fsa, 1).flat_map(|batch| batch) {
    println!("{}", word);
}
```