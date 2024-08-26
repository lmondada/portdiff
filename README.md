# PortDiff

Data structure for fast local graph rewriting. Efficiently store and traverse hierarchies of deltas (diffs) of portgraphs.

### Abstract

Graph rewriting is a powerful and general technique for optimisation problems on graphs. In the quantum computing domain, complete equational theories for quantum circuits provide strong theoretical foundations for rewriting. Unfortunately, rewriting-based circuit optimisation using a naive backtracking search is slow in practice, due to its poor scaling both in the number of rewrite rules and in the input circuit size, as well as being hard to parallelise.

We propose concurrent graph rewriting to address these issues, inspired from equality saturation for term rewriting. Rewrites can be applied in parallel on a persistent data structure. The data structure stores rewrites that can be applied either on the input circuit directly or following a sequence of previous rewrites. After an initial exploration phase, in which all possible rewrites are identified and added to the data structure, an extraction phase determines the set of rewrites that should be applied to optimise the circuit cost function. The exploration phase is designed to scale to large distributed systems, whilst the optimisation problem in the extraction phase can be solved using an off-the-shelf SMT solver.

### Example
TODO