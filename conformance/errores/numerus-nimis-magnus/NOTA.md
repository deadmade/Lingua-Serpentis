# Numerus nimis magnus

D-20 is fixed.

## What the program did

`x = x * x` in an endless loop ran until it was killed, printing nothing and raising no
diagnostic. Each squaring doubles the number of digits and roughly quadruples the time, so the
program was never slow; it asked for a number the machine could not hold. The fixture runner's
backstop would have been the only thing to end it, which is why the register called D-20 "not
fixture-shaped".

## What it does now

A stored number has at most XXXVIII decimal digits. For `mnr` and `nr` that limit is on the grain
count, not on the value. The loop stores 2^64 after its sixth squaring. At the seventh it would
store 2^128, which has thirty-nine digits, so it ends with `numerus nimis magnus est` and exit 1.

The check is in `int_novus`, the function every stored number is boxed by. The diagnostic carries
no line because `liss_int_mul`, like the other arithmetic entry points, takes none. It has the shape
of `memoria exhausta est`.
