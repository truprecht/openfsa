pub mod generator;

use std::rc::Rc;
use std::fmt::{Debug, Display, Error, Formatter};
use std::hash::Hash;
use openfsa_sys::*;
use integeriser::{HashIntegeriser, Integeriser};
use libc::{c_float, c_int};
use log_domain::LogDomain;
use std::borrow::Borrow;
use std::io;

use fsa::generator::BatchGenerator;


/// Transition of an FSA with states of type `Q` and labels of type `A`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Arc<Q, T> {
    pub from: Q,
    pub to: Q,
    pub label: T,
    pub weight: LogDomain<f32>,
}

///  Data type for finite state automata with labels of type `A`.
#[derive(Clone)]
pub struct Automaton<A: Hash + Eq> {
    fsa: Rc<fsa_t>,
    labels: Rc<HashIntegeriser<A>>,
}

impl<T> Automaton<T>
where
    T: Hash + Eq,
{
    /// Hadarmard product of two Automata.
    /// Consumes both Automata and returns an `Automaton` whose language contains
    /// the intersection of both Automata's languages.
    pub fn intersect(&self, other: &Automaton<T>) -> Self {
        Automaton {
            fsa: Rc::new(unsafe {
                fsa_intersect(self.fsa.borrow(), other.fsa.borrow())
            }),
            labels: Rc::clone(&self.labels),
        }
    }

    /// Hadamard product with inverted automaton.
    /// Consumes both Automata and returns an `Automaton` whose
    /// language contains the words contained in the language
    /// of the first `Automaton` minus the words contained in the language of the second one.
    pub fn difference(&self, other: &Automaton<T>) -> Self {
        Automaton {
            fsa: Rc::new(unsafe {
                fsa_difference(self.fsa.borrow(), other.fsa.borrow())
            }),
            labels: Rc::clone(&self.labels),
        }
    }

    // automaton containing the n best words
    fn n_best_automaton(&self, n: usize) -> Self {
        let nbest = unsafe { fsa_n_best(self.fsa.borrow(), n as c_int) };

        Automaton {
            fsa: Rc::new(nbest),
            labels: Rc::clone(&self.labels),
        }
    }

    /// Read an `Automaton` from a binary file.
    pub fn read_binary<R>(labels: Rc<HashIntegeriser<T>>, reader: R) -> io::Result<Automaton<T>>
    where
        R: io::Read,
    {
        let mut rvec: Vec<u8> = {
            let res: io::Result<Vec<u8>> = reader.bytes().collect();
            res?
        };
        let cvec = vec_t::new(&mut rvec);
        Ok(Automaton {
            labels,
            fsa: Rc::new(unsafe { fsa_from_string(&cvec) }),
        })
    }

    /// Dump an `Automaton` to a binary file.
    pub fn write_binary<F>(&self, writer: &mut F) -> io::Result<()>
    where
        F: io::Write,
    {
        let cvec = unsafe { fsa_to_string(self.fsa.borrow()) };
        let slice: &[u8] = cvec.as_slice();
        writer.write_all(slice)
    }

    /// Consume an `Automaton` to construct an `Iterator` that iterates over
    /// all words contained in its language.
    /// Internally, it will repeatedly generate the `step` best words contained in the
    /// language of this `Automaton` and gradually yield those words.
    pub fn generate(self, step: usize) -> BatchGenerator<T> {
        BatchGenerator::new(self, step)
    }
}

impl<T> Automaton<T>
where
    T: Hash + Eq + Display + Clone,
{
    /// Dump the symbol table to tab seperated values.
    pub fn write_symbols<F>(&self, writer: &mut F) -> io::Result<()>
    where
        F: io::Write,
    {
        let labels = Borrow::<HashIntegeriser<T>>::borrow(&self.labels);
        for label_id in 0..(labels.size()) {
            if let Err(e) = write!(
                writer,
                "{}\t{}\n",
                labels.find_value(label_id).unwrap(),
                label_id + 1
            ) {
                return Err(e);
            }
        }
        Ok(())
    }
}

impl<A> Automaton<A>
where
    A: Hash + Eq + Clone,
{
    // constructs a FSA with integerized transition labels
    // uses an existing integerizer to unify labels
    fn from_arcs_with_labels<Q>(
        initial_state: Q,
        final_states: Vec<Q>,
        arcs: Vec<Arc<Q, A>>,
        i_labels: &mut HashIntegeriser<A>,
    ) -> fsa_t
    where
        Q: Hash + Eq + Clone,
    {
        let mut i_states = HashIntegeriser::new();

        // ensure initial state = 0, final state in i_states
        i_states.integerise(initial_state);
        let mut qfs = Vec::new();
        for final_state in final_states {
            qfs.push(i_states.integerise(final_state) as c_int);
        }

        let mut carcs: Vec<fsa_arc> = Vec::new();
        for arc in arcs {
            let Arc {
                from,
                to,
                label,
                weight,
            } = arc;
            carcs.push(fsa_arc {
                from_state: i_states.integerise(from) as c_int,
                to_state: i_states.integerise(to) as c_int,
                label: (i_labels.integerise(label) + 1) as c_int,
                weight: -weight.ln() as c_float,
            });
        }

        unsafe {
            fsa_from_arc_list(
                i_states.size() as c_int,
                &vec_t::new(&mut qfs),
                &vec_t::new(&mut carcs),
            )
        }
    }

    /// Default constructor for an `Automaton`.
    /// Consumes a list of `Arc` transitions and stores all labels
    /// of type `A` in an `Integerizer<A>`.
    /// The original states of type `Q` are lost after integerization.
    pub fn from_arcs<Q>(initial_state: Q, final_state: Vec<Q>, arcs: Vec<Arc<Q, A>>) -> Automaton<A>
    where
        Q: Hash + Eq + Clone,
    {
        // start with one per default, since zero is reserved for epsilon
        let mut integeriser = HashIntegeriser::new();
        let fsa = Rc::new(Automaton::from_arcs_with_labels(
            initial_state,
            final_state,
            arcs,
            &mut integeriser,
        ));

        Automaton {
            fsa,
            labels: Rc::new(integeriser),
        }
    }

    /// Alternative constructor for an `Automaton`.
    /// Synchronizes label integerization using the labels of an existing
    /// `Automaton` and consumes a `Vec`tor of `Arc`s like `from_arcs`.
    /// The `Integerizer` storing the labels of the first `Automaton`
    /// is expanded to store the labels of both Automata.
    pub fn from_arcs_with_same_labels<Q>(
        &self,
        initial_state: Q,
        final_state: Vec<Q>,
        arcs: Vec<Arc<Q, A>>,
    ) -> Automaton<A>
    where
        Q: Hash + Eq + Clone,
    {
        let mut integeriser = (*self.labels).clone();
        let fsa = Rc::new(Automaton::from_arcs_with_labels(
            initial_state,
            final_state,
            arcs,
            &mut integeriser,
        ));

        Automaton {
            fsa,
            labels: Rc::new(integeriser),
        }
    }

    // todo: return arc iterator
    /// Lists the `Arc`s of an `Automaton`.
    /// Since the original type of states cannot be recovered, we use `usize`.
    pub fn into_arcs(self) -> (Vec<Arc<usize, A>>, usize, Vec<usize>) {
        let (carcs, q0, qfs): (Vec<fsa_arc>, c_int, Vec<c_int>) = unsafe {
            let carcs = fsa_to_arc_list(self.fsa.borrow());
            let qi = fsa_initial_state(self.fsa.borrow());
            let qfs = fsa_final_states(self.fsa.borrow());

            (carcs.to_vec(), qi, qfs.to_vec())
        };

        let arcs = carcs
            .into_iter()
            .map(|carc| match carc {
                fsa_arc {
                    from_state,
                    to_state,
                    label,
                    weight,
                } => Arc {
                    from: from_state as usize,
                    to: to_state as usize,
                    label: self.labels
                        .find_value((label - 1) as usize)
                        .unwrap()
                        .clone(),
                    weight: LogDomain::new((-weight).exp()).unwrap(),
                },
            })
            .collect();

        (
            arcs,
            q0 as usize,
            qfs.into_iter().map(|x| x as usize).collect(),
        )
    }
}


use serde::ser::{Serialize, Serializer};
use serde::de::{Deserialize, Deserializer};

impl<T> Serialize for Automaton<T>
where
    T: Serialize + Hash + Eq,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let &Automaton {
            ref fsa,
            ref labels,
        } = self;

        (
            Borrow::<fsa_t>::borrow(fsa),
            Borrow::<HashIntegeriser<T>>::borrow(labels),
        ).serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for Automaton<T>
where
    T: Deserialize<'de> + Hash + Eq + Clone,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Automaton<T>, D::Error> {
        type Tup<T> = (fsa_t, HashIntegeriser<T>);
        let (fsa, labels) = Tup::deserialize(deserializer)?;

        Ok(Automaton {
            fsa: Rc::new(fsa),
            labels: Rc::new(labels),
        })
    }
}

impl<T> Debug for Automaton<T>
where
    T: Debug + Hash + Eq,
{
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(
            f,
            "Automaton {{ fsa: {:?}, labels: {:?} }}",
            self.fsa,
            self.labels
        )
    }
}

impl<T> Display for Automaton<T>
where
    T: Display + Hash + Eq + Clone,
{
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let (arcs, q0, qfs) = self.clone().into_arcs();

        let qfs_strings: Vec<String> = qfs.iter().map(|q| format!("{}", q)).collect();
        let arc_strings: Vec<String> = arcs.iter().map(|arc| format!("{}", arc)).collect();

        write!(
            f,
            "initial {}\nfinal: {}\n{}",
            q0,
            qfs_strings.join(", "),
            arc_strings.join("\n")
        )
    }
}

impl<T, Q> Display for Arc<Q, T>
where
    T: Display,
    Q: Display,
{
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(
            f,
            "{}[{}]\t→ {} # {}",
            self.from,
            self.label,
            self.to,
            self.weight
        )
    }
}



// tests

#[cfg(test)]
mod tests {
    use fsa::*;
    use num_traits::One;

    #[test]
    fn simple_fsa() {
        let arcs = vec![
            Arc {
                from: "q",
                to: "q",
                label: "word",
                weight: LogDomain::new(0.9).unwrap(),
            },
        ];
        let arcs_ = vec![
            Arc {
                from: 0,
                to: 0,
                label: "word",
                weight: LogDomain::new(0.9).unwrap(),
            },
        ];
        let fsa = Automaton::from_arcs("q", vec!["q"], arcs);

        assert_eq!((arcs_, 0, vec![0]), fsa.into_arcs());
    }

    #[test]
    fn simple_intersection() {
        let arcs = vec![
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
        let fsa = Automaton::from_arcs("q1", vec!["q1"], arcs.clone());
        let fsa_ = Automaton::from_arcs("q1", vec!["q1"], arcs);

        let arcs_ = vec![
            Arc {
                from: 0,
                to: 1,
                label: "a",
                weight: LogDomain::new(0.9).unwrap().pow(2.0),
            },
            Arc {
                from: 1,
                to: 0,
                label: "word",
                weight: LogDomain::one(),
            },
        ];

        let intersection = fsa.intersect(&fsa_);

        assert_eq!((arcs_, 0, vec![0]), intersection.into_arcs());
    }

    #[test]
    fn language_generator() {
        let arcs: Vec<Arc<&str, &str>> = vec![
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
        let language: Vec<(Vec<&str>, LogDomain<f32>)> =
            Automaton::from_arcs("q1", vec!["q1"], arcs)
                .generate(2)
                .flat_map(|words| words)
                .take(4)
                .collect();
        let ww = LogDomain::new(0.9).unwrap();
        let words: Vec<(Vec<&str>, LogDomain<f32>)> = vec![
            (Vec::new(), LogDomain::one()),
            (vec!["a", "word"], ww),
            (vec!["a", "word", "a", "word"], ww.pow(2.0)),
            (vec!["a", "word", "a", "word", "a", "word"], ww.pow(3.0)),
        ];

        assert_eq!(words, language);
    }

    #[test]
    fn io() {
        let arcs = vec![
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
        println!("{}", Automaton::from_arcs("q", vec!["q"], arcs.clone()));
        println!("{:?}", Automaton::from_arcs("q", vec!["q"], arcs));
    }
}
