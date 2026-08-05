D-12 is fixed. `longitudo II` was `longitudo 2` — the index was rendered
through `Roman::display()` but the length was interpolated as a bare `usize`,
which made this the one place in the language a user could still see an Arabic
number. roman-native-integers §1 promises "no Arabic number anywhere a user can
see"; this fixture was the counterexample and is now the proof.

An empty list reads `longitudo nullus`, which is how `Roman::display` renders
zero.
