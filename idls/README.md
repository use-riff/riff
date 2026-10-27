# Interfaces of programs riff calls

| File | Program | Address | Source |
|---|---|---|---|
| `dynamic_bonding_curve.json` | Meteora Dynamic Bonding Curve (IDL v0.2.2) | `dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN` | Extracted from Meteora's MIT-licensed SDK, `@meteora-ag/dynamic-bonding-curve-sdk@2.0.0` (`src/idl/dynamic-bonding-curve/idl.json`) |
| `cp_amm.json` | Meteora DAMM v2, where coins graduate (IDL v0.2.5) | `cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG` | Extracted from Meteora's MIT-licensed SDK, `@meteora-ag/cp-amm-sdk@1.5.1` (`src/idl/cp_amm.json`) |

`declare_program!` reads these to generate type-safe CPI calls. Meteora's
program source is under a non-commercial licence, so the compiled programs are
not committed: the tests download them (`make dbc-fixtures`).
