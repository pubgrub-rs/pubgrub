# How can I contribute? Here are some ideas

- Use it!
  Indeed there is quite some work left for custom
  dependency providers. So just using the crate, building
  you own dependency provider for your favorite programming language
  and letting us know how it turned out
  would be amazing feedback already!

- Non failing extension for multiple versions.
  Currently, the solver works by allowing only one version per package.
  In some contexts however, we may want to not fail if multiple versions are required,
  and return instead multiple versions per package.
  Such situations may be for example allowing multiple major versions of the same crate.
  But it could be more general than that exact scenario.
