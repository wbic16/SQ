* Demos
  * Exocortical Minds
  * REST throughput fetching database records
  * daemon throughput streaming thoughts to disk
  * DNA Analysis
  * Phext Widgets
  * Merging content at scale
  * Disk Bandwidth Scaling
  * phext arena
    * phext vs text (EZ Mode)
    * phext vs tar (readable archives!?)
    * phext vs JSON (scalability and error-handling)
    * phext vs XML (markup is slow)
    * phext vs YAML (finish it!)
* integrate hierarchical content hashing (see libphext)
* concurrent benchmarks
* rebase hello-phext upon sq
* support multi-GB scrolls
* support phexts spanning disks
* mind map (prefix firewalling)
  * example map:
  * 1.x.x/*/*: self
  * 2.x.x/*/*: play
  * 3.x.x/*/*: spouse
  * 4.x.x/*/*: howto
  * 5.x.x/*/*: work
  * 6.x.x/*/*: knowledge
  * 7.x.x/*/*: science
* mind meld API
  * request-response cycles
  * dopamine training hacks
## wikibench targets (phext-lattice/bench/wiki/FINDINGS.md, 38k-article / 49 MB phext)
* HTTP keep-alive: `select` is 86 µs p50 over HTTP vs 0.7 µs in-process; one TCP connection per request dominates
* durable, O(edit) mutations: `update` takes 72 ms because every mutation implodes and rewrites the whole phext, and it never fsyncs
* server-side link expansion: a 2-hop BFS through select + /api/v2/query takes 778 µs vs 43 µs in-process; add a links/depth query
* cold start: first request takes 171 ms (explode into HashMap<Coordinate, String>); host on phext-lattice's MappedLattice instead
* /api/v2/query ranges scan the whole map; a sorted span index makes them O(log n + k)
