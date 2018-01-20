# GaryDB: Single File key:value Store of Arbitrary Depth
> Note: this is litterally at Alpha Alpha stage and is not usable.

GaryDB is a single file database in the vein of SQLite or NeDB.

Some reasons you might want to support GaryDB:
- Written in 100% rust with very few dependencies.
- Generalized: Can handle tables of arbitrary depth (tables within tables) and
  with arbitrary types. (Non-table) values are stored as binary data, which
  makes use of `serde::{Serialize, Deserialize}` traits to store/retrieve them.
- Persistent: stored as a single file to disk
- API is through it's exported types/functions. Has no (built in) protocol,
  instead it works as a library in your application and operations are
  guaranteed memory+thread safe through the rust type system.
- Concurrent: can have multiple readers/writers with the concept of
  a very fast "head" writer that is pushing progressivey larger values.
  Good for logging and nested-logging.
- Scalable: about as scalable as a single file database written by an amateur
  can get.
