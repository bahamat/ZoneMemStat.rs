# zonememstat.rs

`zonememstat.rs` is a rust interface to `zonememstat` on SmartOS.

Calling `zonememstat::stat()` will return a Vec of ZoneMemStat structs
which in turn represents the output columns from `zonememstat -a`.
