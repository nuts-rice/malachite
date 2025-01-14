# Tendermint Consensus Algorithm

A consensus algorithm is run by a set of **processes**[^1] (some of which may
fail) that **propose** and **decide** values. The algorithm guarantees that 
eventually all correct processes **decide** the same value, which must be one of
the proposed values.

[Tendermint][tendermint-arxiv] is a Byzantine Fault-Tolerant (BFT) consensus
algorithm, designed to tolerate the most comprehensive set of faulty behaviours.
A Byzantine process is a faulty process that can operate arbitrarily;
this means that in addition to the benign failures (crash, omission, etc.),
the process can, deliberately or not, disregard the rules imposed by the
algorithm.
Tendermint can solve consensus as long as **less than one third of the
processes are Byzantine**.

Tendermint assumes that processes interact by exchanging messages.
All consensus messages are assumed to include a **digital signature**.
This enables a process receiving a message to authenticate its sender and
content.
Byzantine nodes are assumed to not to be able to break digital signatures,
that is, they cannot forge messages and impersonate correct senders
(a.k.a. non-masquerading).

In the remainder of this document, the main concepts and the overall operation
of Tendermint are detailed.

## Heights

The algorithm presented in the [pseudo-code][pseudo-code] represent the
operation of an instance of consensus in a process `p`.

Each instance or **height** of the consensus algorithm is identified by an
integer, represented by the `h_p` variable in the pseudo-code.
The height `h_p` of consensus is finalized when the process reaches a decision
on a value `v`, represented in the pseudo-code by the action
`decision_p[h_p] = v` (line 51).
A this point, the process increases `h_p` (line 52) and starts the next height
of consensus, in which the same algorithm is executed again.

For the sake of the operation of the consensus algorithm, heights are
completely independent executions.
For this reason, in the remainder of this document we consider and discuss the
execution of a **single height of consensus**.

## Rounds

A height of consensus is organized into rounds, identified by integers and
always starting from round 0.
The round at which a process `p` is identified in the
[pseudo-code][pseudo-code] by the `round_p` variable.

A **successful round** leads the process to decide on a value proposed in that
round, as in the pseudo-code block starting from line 49.
While in an **unsuccessful round** the process does not decide a value and
move to the next round, as in the pseudo-code block of line 65,
or skip to an arbitrary higher round, as in the pseud-code block of line 55.

The execution of each round of consensus is led by a process selected as the
**proposer** of that round.
Tendermint assumes the existence of a [`proposer(h, r)`](#proposer-selection) function
that implements a deterministic proposer selection algorithm and returns the
process that should be the proposer and lead round `r` of consensus height `h`.

## Round Steps

A round of consensus is organized into a sequence of three round steps:
`propose`, `prevote`, and `precommit`, as defined in line 4 of the
[pseudo-code][pseudo-code].
To each round step is associated a [consensus message](#messages), exchanged by
processes during that round step.

The current round step of a process `p` is stored in the `step_p` variable.
In general terms, when entering a round step, a process performs one or more
**actions**.
And the reception a given set of **events** while in a round step, leads the
process to move to the next round step.

### Propose

The `propose` round step is the first step of each round.
A process `p` sets its `step_p` to `propose` in the `StartRound(round)`
function, where it also increases `round_p` to the started `round` number.
The `propose` step is the only round step that is asymmetric: different
processes perform different actions when starting it.
More specifically, the proposer of the round has a distinguish role in this round step.

In the `propose` round step, the **proposer** of the current round selects the
value to be the proposed in that round and **broadcast**s the proposed value to all
processes (line 19), in a [`PROPOSAL`](#proposals) message.
All other processes start a **timeout** (line 21) to limit the amount of time
they wait for the `PROPOSAL` message sent by the proposer.

### Prevote

The `prevote` round step has the role of validating the value proposed in the
`propose` step.
The value proposed for the round can be accepted (lines 24 or 30) or rejected
(lines 26 or 32) by the process.
The proposed value can be also rejected if not received when the timeout
scheduled in the `propose` step expires (line 59).

The action taken by a process when it moves from the `propose` to the `prevote`
step is to **broadcast** a message to inform all processes whether it has accepted
or not the proposed value.
To accept a value `v`, the process issues a `PREVOTE` message for `id(v)`;
in contrast, to reject it, it issues a `PREVOTE` message for the special `nil` value.

The remaining of this step consists of collecting the `PREVOTE` messages that
processes have broadcast in the same round step.
In the case where there is no agreement on whether the value proposed on the
current round is acceptable or not, the process schedules a **timeout** (line
35) to limit the amount of time it spends in this round step.

### Precommit

The `precommit` round step is when it is defined whether a round of consensus
has succeeded or not.
In the case of a successful round, the decision value has been established and
it is committed: the consensus height is done (line 51).
Otherwise, the processes will need an additional round to attempt reaching a
decision (line 67).

The action taken by a process when it moves from the `prevote` step to the
`precommit` step is to **broadcast** a message to inform whether an agreement
has been observed in the `prevote` round step (lines 40, 45, or 63).
If an agreement was observed around a value `v`, the process issues a
`PRECOMMIT` message for `id(v)`;
otherwise, it issues a `PRECOMMIT` message for the special `nil` value.

The remaining of this step consists of collecting the `PRECOMMIT` messages that
processes have broadcast in the same round step.
If there is conflicting information on the messages received from a
super-majority of processes, the process schedules a **timeout** (line 48) to
limit the amount of time it waits for the round to succeed;
if this timeout expires before a decision is reached, the round has failed
(line 67).

### Exit Conditions

A peculiarity of the actions associated to the `precommit` round step is that
a process `p` does not need to have `step_p = precommit` to perform them.
This happens because this round step concludes a round, and possibly a height
when a decision is reached, so that the associated actions can be performed at
any time when the **exit conditions** are observed:

- If a process `p` is at any round step of round `round_p` and the conditions
  from line 47 of the pseudo-code are observed (i.e., conflicting `PRECOMMIT`
  messages for round `round_p`), the process schedules a **timeout** for the
  `precommit` round step (line 48);
- If the above mentioned scheduled **timeout** for the `precommit` round step
  expires in a process `p` that still has `round_p = round` and `h_p == height`,
  the current round `round_p` has failed and the process starts the next round (line 67);
- If a process `p` observes the conditions from line 49 of the pseudo-code
  (i.e., agreeing `PRECOMMIT` messages for a proposed value `v`) for
  **any round** `r` of its current height `h_p`, the decision value `v` is
  committed and the height `h_r` of consensus is finalized.

Notice that in the last case, the commit round `r` can be the current round
(`r = round_p`), a previous round (`r < round_p`) that the process deemed
failed, or a future round (`r > round_p`) that the process has not yet started.

## Messages

The Tendermint consensus algorithm defines three message types, each type
associated to a [round step](#round-steps):

- `⟨PROPOSAL, h, r, v, vr⟩`: broadcast by the process returned by the proposer
  selection function `proposer(h, r)` function when entering the
  [`propose`](#propose) step of round `r` of height `h`.
  Carries the proposed value `v` for height `h` of consensus.
  Since only proposed values can be decided, the success of round `r` depends
  on the reception of this message.
- `⟨PREVOTE, h, r, *⟩` broadcast by all processes when entering the
  [`prevote`](#prevote) step of round `r` of height `h`.
  The last field can be either the unique identifier `id(v)` of the value `v`
  carried by a `⟨PROPOSAL, h, r, v, *⟩` message, meaning that it was received
  and `v` has been accepted by the process, or the special `nil` value otherwise.
- `⟨PRECOMMIT, h, r, *⟩`: broadcast by all processes when entering the
  [`precommit`](#precommit) step of round `r` of height `h`.
  The last field can be either the unique identifier `id(v)` of a proposed
  value `v` for which the process has received `⟨PREVOTE, h, r, id(v)⟩`
  messages from a super-majority of processes, or the special `nil` value otherwise.

Before discussing in detail the role of each message in the protocol, it is
worth highlighting a main aspect that differentiate the adopted messages.
The `PROPOSAL` message is assumed to carry a proposed value `v`, which may have
an arbitrary size; we refer to it as the "full" value `v`.
The `PREVOTE` and `PRECOMMIT` messages, generally called [votes](#votes), are
expected to be fixed size and much smaller than `PROPOSAL` messages.
This is because they do not carry the "full" proposed value `v`, but instead an
unique identified `id(v)` of a proposed value `v`.

The propagation of large values, included in `PROPOSAL` messages, in practice
requires specific and efficient data dissemination protocols.
Implementations typically split the `PROPOSAL` message into multiple parts,
independently propagated and reconstructed by processes.

## Proposals

Proposals are produced and broadcast by the `StartRound(round)` function of the
[pseudo-code][pseudo-code], by the process selected as the **proposer** of
the started round.
The proposer of the current round is returned to process `p` by the
`proposer(h_p, round_p)` function.

Every process expects to receive the `⟨PROPOSAL, h, r, v, *⟩` broadcast by
`proposer(h, r)`, as its reception is a condition for all state transitions
that propitiate a successful round `r`, namely the pseudo-code blocks starting
from lines 22 or 28, 36, and 49.
The success of round `r` results in `v` being the decision value for height `h`.

### Value Selection

The proposer of a round `r` defines which value `v` it will propose based on
the values of the two state variables `validValue_p` and `validRound_p`.
They are initialized to `nil` and `-1` at the beginning of each height, meaning
that the process is not aware of any proposed value that has became **valid**
in a previous round.
A value `v` becomes **valid** at round `r` when a `PROPOSAL` for `v` and an
enough number of `PREVOTE` messages for `id(v)` are received during round `r`.
This logic is part of the pseudo-code block from line 36, where `validValue_p`
and `validRound_p` are updated.

If the proposer `p` of a round `r` of height `h` has `validValue_p != nil`,
meaning that `p` knows a valid value, it must propose that value again.
The message it broadcasts when entering the `prevote` step of round `r` is
thus `⟨PROPOSAL, h, r, validValue_p, validRound_p⟩`.

If the proposer `p` of a round `r` of height `h` has `validValue_p = nil`, `p`
may propose any value it wants.
The function `getValue()` is invoked and returns a value to be proposed.
The message it broadcasts when entering the `prevote` step of round `r` is
thus `⟨PROPOSAL, h, r, getValue(), -1⟩`.
Observe that this is always the case in round 0 and the most common case in
ordinary executions.

### Byzantine Proposers

A correct process `p` will only broadcast a `⟨PROPOSAL, h, r, v, vr⟩` message
if `p = proposer(h, r)`, i.e., if it is the proposer of the started round `r`.
It will follow value selection roles and broadcast a single `PROPOSAL` message.

A Byzantine process `q` may not follow any of the above mentioned algorithm
rules. More precisely, it can perform the following **attacks**:

1. `q` may broadcast a `⟨PROPOSAL, h, r, v, vr⟩` message while `q !=  proposer(h, r)`;
2. `q` may broadcast a `⟨PROPOSAL, h, r, v, -1⟩` message while `v != validValue_q != nil`;
3. `q` may broadcast a `⟨PROPOSAL, h, r, v, vr⟩` message while `-1 < vr != validRound_q`;
4. `q` may broadcast multiple `⟨PROPOSAL, h, r, *, *⟩` messages, each proposing a different value.

Attack 1. is simple to identify and deal, since proposals include the
**digital signature** of their senders, and the
[proposers](#proposer-selection) for any given round of a height are assumed to
be a priori known by all consensus processes.

Attacks 2. and 3. are constitute forms of the **amnesia attack** and are harder
to identify.
Notice, however, that correct processes check whether they can accept a proposed
value `v` with valid round `vr` based in the content of its state variables
`lockedValue_p` and `lockedRound_p` (lines 23 and 29) and are likely to reject
such proposals.

Attack 4. constitutes a double-signing or **equivocation** attack.
The most common approach for a correct process is to only consider the first
`⟨PROPOSAL, h, r, v, *⟩` received in the `propose` step, which can be accepted
or rejected.
However, it is possible that a different `⟨PROPOSAL, h, r, v', *⟩` with
`v' != v` is accepted by different processes and, as a result, triggers state
transitions in the `prevote` or `precommit` round steps.
So, a priori, the algorithm expects a correct process to store all the
multiple proposals broadcast by a Byzantine proposer.
Which, by itself, constitutes an attack vector to be considered.

While hard to handle, it is easy to prove that a process has performed an
equivocation attack: it is enough to receive and store distinct messages for
the same height, round, and round step signed by the same process.
For a more comprehensive discussion on misbehavior detection, evidence
production and dissemination refer to this [document](./misbehavior.md).

## Votes

Vote is the generic name for `⟨PREVOTE, h, r, *⟩` and `⟨PRECOMMIT, h, r, *⟩` messages.
Tendermint includes two voting steps, the `prevote` and the `precommit` round
steps, where the corresponding votes are exchanged.

Differently from proposals, that are broadcast by the proposer of a round to
all processes (1-to-n communication pattern), every process is expected to send
its votes, two votes per round, to all processes (n-to-n communication
pattern).
However, while proposals carry a (full) proposed value `v`, with variable size,
votes only carry a (probably fixed-size and small) unique identifier `id(v)` of
the proposed value, or the special value `nil` (which means "no value").

Moreover, the analysis of the [pseudo-code][pseudo-code] reveals that, while
the reception of a proposal is considered by itself an event that may trigger a
state transition, the reception of a _single_ vote message does not by itself
trigger any state transition.
The main reason for that is the fact that up to `f` processes are assumed to be
Byzantine and can produce arbitrary vote messages.
As a result, no information produced by a single, or by a set with at most `f`
processes can be considered legit and should not drive the operation of correct
processes.

### Voting power

Up to this point, this document is aligned with the pseudo-code and has the
following failure assumptions:

1. The algorithm tolerates `f` Byzantine-faulty processes, which may behave
   arbitrarily;
2. The algorithm requires that less than one third of the processes are
   Byzantine. So, if `n` is the total number of processes, the algorithm
   assumes that `f < n/3` processes can be Byzantine. It then considers a
   minimal set of `n = 3f + 1` processes.

This failure model is built from the common assumption that processes are
homogeneous, in the sense that the vote of any process counts the same: one
process, one vote.
In other words, all processes have the same voting power.

Tendermint was designed to support the operation of Proof-of-Stake (PoS)
blockchains, where processes are assumed to stake (deposit) some amount to be
active actors in the blockchain and to have a voting power that is proportional
to the staked amount.
In other words, when adopting the PoS framework, processes are assumed to have
distinct voting powers.
The failure assumptions are thus updated as:

1. Each process `p` owns or has an associated voting power `p.power > 0`;
2. The system is composed by a set of process whose aggregated or total voting
   power is `n`;
3. The maximum voting power owned by or associated to Byzantine processes is
   assumed to be `f < n/3`.

This means, in particular, that when `f + 1` is used in the pseudo-code, it
must be considered a set of processes whose aggregated voting power is strictly
higher than `f`, i.e., strictly higher than `1/3` of the total voting power `n`
of all processes.
The use of this _threshold_ means that, among the considered processes,
**at least one process is correct**.

Analogously, when `2f + 1` is used in the pseudo-code, it must be interpreted
as a set of processes in which the aggregated voting power of correct processes
in the set is strictly higher than the aggregated voting power of (potentially)
Byzantine processes in the set.
The use of this _threshold_ means that **the majority of the considered processes is correct**.

### Byzantine Voters

A correct process `p` will broadcast at most one `⟨PREVOTE, h, r, *⟩` and at
most one `⟨PRECOMMIT, h, r, *⟩` messages in round `r` of height `h`.
The votes `p` broadcasts in each round step will carry either the unique
identifier `id(v)` of the value `v`, that `p` has received in a `⟨PROPOSAL, h, r, v, *⟩`
message from `proposer(h, r)`, or the special value `nil`.

Byzantine processes, however, can broadcast multiple vote messages for the same
round step, carrying the identifier of any value or the special `nil` value.
The main attacks that are worth considering, because of their potential of
inducing undesirable behaviour, are two:

1. **Equivocation**: a Byzantine process can broadcast multiple
   `⟨PREVOTE, h, r, *⟩` or `⟨PRECOMMIT, h, r, *⟩` messages in the same round
   `r` of height `h`, for distinct values: `nil`, `id(v)`, or `id(v')`
   with `v != v'`.
2. **Amnesia**: a Byzantine process `q` can broadcast `⟨PREVOTE, h, r, *⟩` or
   `⟨PRECOMMIT, h, r, *⟩` messages for values that are not in line with the
   expected contents of its `lockedValue_q` and `lockedRound_q` variables.

Since Byzantine processes can always produce **equivocation attacks**, a
correct process can deal with them is by only considering the first
`⟨PREVOTE, h, r, *⟩` or `⟨PRECOMMIT, h, r, *⟩` messages received from a process
in a round `r` of height `h`.
Different (equivocating) versions of the same message from the same sender
should, from a defensive point of view, be disregarded and dropped by the
consensus logic as they were duplicated messages.
The reason for which is the fact that a Byzantine process can produce an
arbitrary number of such messages, therefore store all of them may constitute
an attack vector.

Unfortunately, there are multiple scenarios in which correct processes may
receive equivocating messages from Byzantine voters in different orders, and
by only considering the first received one, they may end up performing
different state transitions in the consensus protocol.
While this does not pose a threat to the safety of consensus, this might
produce liveness issues, as correct processes may be left behind in the
consensus computation.
See this [discussion](https://github.com/informalsystems/malachite/discussions/380)
for some examples.

The **amnesia attack** is also virtually impossible to prevent and it is also
harder to detect than equivocation ones.
A correct process `p` that broadcasts a `⟨PRECOMMIT, h, r, id(v)⟩` must update
its variables `lockedValue_p ← v` and `lockedRound_p ← r`, as shown in the
pseudo-code block from line 36.
From this point, `p` is locked on value `v`, which means that upon receiving a
`⟨PROPOSAL, h, r', v', vr⟩` message for a round `r' > r`, it must reject the
proposed value `v'` if it does not match its locked value `v`, i.e., it must
broadcast a `⟨PREVOTE, h, r', nil⟩` message.
The only possible exception is when the proposal's valid round `vr > r`
corresponds to a round where there was an agreement on the proposed value
`v' != lockedValue_p`, as shown in the pseudo-code block from line 28.
In this case, issuing a `⟨PREVOTE, h, r', id(v')⟩` can be justified as a
correct behaviour provided that `p` is able to prove the existence of a
`2f + 1 ⟨PREVOTE, h, vr, id(v')⟩` set of messages accepting `v'`.

> A variation of Tendermint consensus protocol, known as
> [Accountable Tendermint][accountable-tendermint], proposes some changes in
> the algorithm to render it possible to detect and produce evidence for the
> amnesia attack (see also [#398](https://github.com/informalsystems/malachite/issues/398)).

## Functions

The [pseudo-code][pseudo-code] of the consensus algorithm includes calls to
functions that are not defined in the pseudo-code itself, but are assumed to be
implemented by the context/application that uses the consensus protocol.

### Proposer Selection

The `proposer(h, r)` function receives a height `h >= 0` and a round `r >= 0`
and returns the process, among the processes running height `h`, selected as
the proposer of round `r` of height `h`.
The role of the proposer is described in the [`propose`](#propose) round step.

The `proposer(h, r)` function requires the knowledge of the set of processes
running the height `h` of consensus.
The set of processes running a given height of consensus is fixed:
it cannot vary over rounds.
But different heights on consensus may be run by distinct set of processes and
processes may have distinct associated [voting powers](#voting-power) in different heights.

#### Determinism

Given a consensus height `h` and the set of processes running height `h`,
the `proposer(h, r)` function **must be deterministic**.
This means that any two correct processes that invoke `proposer(h, r)` with the
same inputs, including the implicit input that is the set of processes running
consensus height `h` and associated voting powers, should receive the exactly
same output (process).

#### Correctness

The main goal of the `proposer(h, r)` function is to eventually select a
correct process to coordinate a round of consensus and to propose an
appropriate value for it.
A correct implementation of the function must guarantee that, for every height
`h`, there is a round `r* >= 0` where `proposer(h, r*)` returns a process `p`
that is a correct process: `p` does not misbehave nor crash.

Fortunately, it is relatively simple to produce a correct implementation of the
`proposer(h, r)` method: it is enough to ensure that every process running
consensus height `h` is selected as the proposer for at least one round.
In other words, it is enough that processes take turns as the proposers of
different rounds of a height.

#### Fairness

Tendermint is a consensus algorithm that adheres to the _rotating coordinator_
approach.
This means that the role of coordinating rounds and proposing values is
expected to be played by different processes over time, in successive
heights of consensus.
(In contrast to the _fixed coordinator_ approach, where the coordinator or
proposer is only replaced when it is suspected to be faulty).

While a correct `proposer(h, r)` implementation eventually selects every
process as the proposer of a round of height `h`, being the proposer of the
first round of a height is the most relevant role, since most heights of
consensus are expected to be finalized in round 0.
A fair proposer selection algorithm should therefore ensure that all processes
have, over a reasonable long sequence of heights `h`, a similar chance of being
selected as `proposer(h, 0)`, thus to propose the value that is most likely to be
decided on that height.

In the case of Proof-of-Stake (PoS) blockchains, where processes are assumed to
have distinct voting powers, a fair proposer selection algorithm should ensure
that every process is chosen,  over a reasonable long sequence of heights, as
`proposer(h, 0)` in a number of heights `h` that is proportional to its voting
power.
Observe that when the set of processes running consensus varies over heights,
so as the voting power associated to each process, producing a fair proposer
selection algorithm becomes more challenging.

> The [proposer selection procedure of CometBFT][cometbft-proposer], which
> implements Tendermint in Go, is a useful reference of how a fair proposer
> selection can be achieved with a dynamic set of processes and voting powers.
> It is worth noting that this algorithm maintains an internal state, which is
> updated whenever a new proposer is selected.
> This means that reproducing the algorithm output requires, in addition to the
> inputs mentioned in the [Determinism section](#determinism), retrieving or
> recomputing the associated algorithm's internal state.

### Proposal value

The external function `getValue()` is invoked by the proposer of a round as
part of the transition to the [propose round step](#propose).
It should return a value to be proposed by the process `p` for the current
height of consensus `h_p`.

> TODO: synchronous/asynchronous implementations, currently discussed
> [here](../english/consensus/README.md#asynchronous-getvalue-and-proposevaluev).

### Validation

The external function `valid` is used to guard any value-specific action in 
the algorithm. The purpose is to restrict the domain of values that consensus 
may decide. There is the assumption that if `v` is a value returned
by the  [`getValue()` function](#proposal-value) of a correct process in the current height, then
`valid(v)` evaluates to `true`. Under this assumption, as long as the proposer
is correct, `valid(v)` does not serve any purpose. It only becomes relevant if
there is a faulty proposer; it limits "how bad" proposed values can be.

The [pseudo-code][pseudo-code] uses `valid(v)`, that is, a function that
maps a value to a boolean. Observe that this should be understood in terms
of a mathematical (pure) function, with the following properties:
1. the function must only depend on `v` but not on other data 
(e.g., an application state at a given point in time, the current local time 
or temperature).
2. If invoked several times for the same input, the function always returns 
the same boolean value (determinism).
3. If invoked on different processes for the same input, the function always 
returns the same boolean value. 

The correctness of the consensus algorithm depends heavily on these points. 
Most obviously for termination, we require that all correct processes find 
the value proposed by a correct process valid under all circumstances, because we
need them to accept the value. A deviation in `valid` would 
result in some processes
rejecting values proposed by correct processes, and consequently, there might never be enough votes to decide a value.

#### Implementation

Implementations of the validity check are typically not pure:
as already mentioned in the [Tendermint paper][tendermint-arxiv], "In the context of blockchain systems, 
for example, a value is not valid if it does not
contain an appropriate hash of the last value (block) added to the blockchain." That is, 
rather than `valid(v)` the implementation uses
the state of the blockchain at a given height and a value, that is, it 
looks something like `valid(v, chain, height)`. (In the [Tendermint paper][tendermint-arxiv],
the data about the blockchain state, that is, past decisions etc., is captured in `decision_p`.) 
This implies that
a value that might be valid at blockchain height 5, might be invalid at height 6. Observe 
that this, strictly speaking, the above defined property 1.
However, as long as all processes agree on 
the state of the chain up to height 5 when they 
start consensus for height 6, this still satisfies properties 2 and 3 and it will not harm 
liveness. (There have been cases of 
non-determinism in the application level that led to processes disagreeing on the 
application state and thus consensus being blocked at a given height)

> **Remark.** We have seen slight (ab)uses of valid, that use external data. Consider he toy 
example of the proposer proposing the current room temperature, and the processes 
checking in `valid` whether the temperature is within one degree of their room 
temperature. It is easy to see that this a priori violates the points 1-3 above. 
One the one hand, this requires additional assumption on the environment to 
ensure termination, on the other hand, it is impossible to check validity after 
the fact (e.g., a late joiner cannot check validity of a value that processes have 
decided some time ago). So this use is not encouraged, and we ignore it in the 
remainder. However, for some applications this use case might still be beneficial: in
this case it is important to understand that one needs to make an argument (which
also needs to involve
the environment) why it
can be ensured that if `v` is a value returned
by the  `getValue()` function of a correct process in the current height, then
`valid(v)` evaluates to `true`.

> **Remark.** Point 1 above also forbids that the current consensus round influences
validity. For instance, one may say to get out of the liveness issue from the previous
remark, to just find all blocks valid that have been proposed after, say, round 10. 
Similarly, one might consider more stricter validity requirements for the proposal in
round 0 (the lucky path), while in unlucky situations one might want to simplify 
reaching a decision by weakening validity. In general this leads to complexity down
the road when someone reads the decisions much later, and needs to understand the 
different semantics of different blocks based on the decision round. We strongly
suggest to not use round numbers in validation of values for this reason.

#### Backwards compatibility

**Requirement 1 (Fixing bugs).**
There might be a bug in the implementation of `valid(v)`. Then it might be possible that due to a bug,
`valid` returns `false` 
for values proposed by correct processes, and we are stuck at a given height. 
A way to get out of the 
situation is to produce a new implementation of `valid(v)` that returns `true` for the values proposed by correct processes.
To be prepared for such a scenario we need to allow a change in the function.

**Requirement 2 (Future use).**
If we allow changes to `valid`, we need to understand all uses of this function. Some
synchronization protocols may use `valid(v)` for consistency checks, for instance, if a node
fell behind, it might need to learn several past decisions. In doing so, it typically also
uses (the current version) of `valid(v)` to check the decided values before accepting them.
In this scenario, a value decided in the past (potentially using a now old version of 
`valid(v)`) should be deemed valid with the current version of the function. 

These two requirements lead us to the following requirement on the implementations:
we consider a sequence of `valid_i(v)` implementations, with increasing versions `i`, 
so that to represent multiple _backwards compatible_ implementations of the validity checks.
Formally we require that
`valid_i(v) == true` implies `valid_j(v) == true `, for `j > i` and every value `v`.

The logical implication allows newer versions of `valid(v)` to be more permissive (as required to
fix bugs), while ensuring that newer versions allow to check validity
of previously decided values.

>**Remark.** A similar way to address these concerns in implementations has been discussed in the
context of soft upgrades [here](
https://github.com/informalsystems/malachite/issues/510#issuecomment-2589858811).



## Primitives

The [pseudo-code][pseudo-code] of the consensus algorithm invokes some
primitives that are not defined in the pseudo-code itself, but are assumed to
be implemented by the processes running the consensus protocol.

### Network

The only network primitive adopted in the pseudo-code is the `broadcast`
primitive, which should send a given [consensus message](#messages) to all
processes, thus implementing a 1-to-n communication primitive.

> TODO: reliable broadcast properties needed for consensus messages, and the
> more comprehensive and strong properties required for certificates (sets of
> 2f + 1 identical votes), and certified proposals.

### Timeouts

The `schedule` primitive is adopted in the pseudo-code to schedule the
execution of `OnTimeout<Step>(height, round)` functions, where `<Step>` is one
of `Propose`, `Prevote`, and `Precommit` (i.e., the three [round steps](#round-steps)),
to the current time plus the duration returned by the corresponding functions
`timeout<Step>(round)`.

> TODO: assumptions regarding timeouts, they should increase over time, GST, etc.

> TODO: most timeouts can be cancelled when the associated conditions are not
> any longer observed (round or height changed, round step changed).

[^1]: This document adopts _process_ to refer to the active participants of the
  consensus algorithm, which can propose and vote for values. In the blockchain
  terminology, a _process_ would be a _validator_. In the specification both
  names are adopted and are equivalent.

[pseudo-code]: ./pseudo-code.md
[tendermint-arxiv]: https://arxiv.org/abs/1807.04938
[accountable-tendermint]: ./misbehavior.md#misbehavior-detection-and-verification-in-accountable-tendermint
[cometbft-proposer]: https://github.com/cometbft/cometbft/blob/main/spec/consensus/proposer-selection.md
