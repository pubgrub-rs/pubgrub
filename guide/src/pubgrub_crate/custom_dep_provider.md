# Writing your own dependency provider

The `OfflineDependencyProvider` is very useful for testing
and playing with the API, but would not be usable in more complex
settings like Cargo for example.
In such cases, a dependency provider may need to retrieve
package information from caches, from the disk or from network requests.

TODO: waiting on potential API changes for `DependencyProvider`.
