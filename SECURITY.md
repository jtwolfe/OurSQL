# Security

A live leak is not a party congress. Do not file it on the public square.

## Live leaks (private)

Use GitHub's private advisory form on this repo:

https://github.com/jtwolfe/OurSQL/security/advisories/new

Include `oursqld --version`, intensity, and a repro that does not need
production rows. Private vulnerability reporting is on.

If that form is closed, you still do not post the exploit as an issue.
Hold it, or send a patch without the war story.

## Everything else: lodge an issue, then do it yourself

The kollektiv does not staff a Ministry of Bug Bounties. No commissar
will seize your stack trace and return a medal. Communism, in this
tree, means you keep the means of data hosting **and** the means of
the patch.

1. Open an issue: https://github.com/jtwolfe/OurSQL/issues/new
2. ASCII title. What broke, at what intensity, how to reproduce.
3. Then fork. Branch `petition/<short-ascii>`. `cargo fmt` and
   `cargo test --workspace`. Pull request. See [CONTRIBUTING.md](CONTRIBUTING.md).

Filing without a patch is allowed. It is also how work waits in the
queue until the five-year plan notices it. The fastest path is still
you.

## Rules

- No "we encrypted your files" jokes. We will assume you are serious
  and then be disappointed.
- We do not pay a bug bounty in a currency this project refuses to mint.
- Satire stays in bureau. A hole in WAL, authz, or mesh is not a bit.

## Scope

In: oursqld, oursql CLI, wire protocol, NashCQL parser, bureau
bypasses that skip authz, WAL corruption that forges certified rows,
unauthenticated mesh APPLY.

Out: social engineering of a komitet, intensity-0 being "not
oppressive enough," keyword taste.
