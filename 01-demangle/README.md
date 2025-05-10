# 1) Demangle

The main goal of this experiment is to demonstrate that we DO NOT have to rely
on #[unsafe(no_mangle)] when dynamically loading symbols!

We achieve this by reading the library's symbol table and finding the symbols
that belong directly to the library be demangling them via rustc_demangle. This
allows us to locate the correct symbol regards of its name, even if the name
were to change across different compiles.

We prove this works by dynamically reloading a simple function that prints
something and showcasing that we can change what it prints dynamically without
recompiling the binary
