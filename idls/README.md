# Interfaces of programs riff calls

| File | Program | Address | Source |
|---|---|---|---|
| `dynamic_bonding_curve.json` | Meteora Dynamic Bonding Curve (IDL v0.2.1) | `dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN` | Extracted from Meteora's MIT-licensed SDK, `@meteora-ag/dynamic-bonding-curve-sdk@1.5.13` (`src/idl/dynamic-bonding-curve/idl.json`) |

`declare_program!` reads these to generate type-safe CPI calls. Meteora's
program source is under a non-commercial licence, so its compiled program is
not committed: the tests download it (`make dbc-fixtures`).
