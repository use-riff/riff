/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/riff_dbc.json`.
 */
export type RiffDbc = {
  "address": "gATYd3vfHCPCrhFupj8HxVX78syyXNfPSbYRsttm9RF",
  "metadata": {
    "name": "riffDbc",
    "version": "0.0.1",
    "spec": "0.1.0",
    "description": "riff on Meteora's Dynamic Bonding Curve: artist fees held for the artist's passport"
  },
  "instructions": [
    {
      "name": "claimArtist",
      "discriminator": [
        18,
        47,
        7,
        231,
        18,
        153,
        154,
        173
      ],
      "accounts": [
        {
          "name": "wallet",
          "docs": [
            "The passport's wallet. Also pays the rent of `unwrap`, and gets it back."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "verifier",
          "signer": true
        },
        {
          "name": "config",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "coin",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  105,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "baseMint"
              }
            ]
          }
        },
        {
          "name": "passport",
          "docs": [
            "The passport of this coin's artist: its seeds hold the artist ID."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  97,
                  115,
                  115,
                  112,
                  111,
                  114,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "coin.artistId",
                "account": "coin"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                26,
                148,
                42,
                179,
                119,
                2,
                217,
                31,
                193,
                66,
                183,
                207,
                194,
                169,
                119,
                15,
                254,
                110,
                10,
                192,
                165,
                86,
                153,
                15,
                146,
                254,
                228,
                107,
                227,
                90,
                61,
                98
              ]
            }
          }
        },
        {
          "name": "passportVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "passport"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                26,
                148,
                42,
                179,
                119,
                2,
                217,
                31,
                193,
                66,
                183,
                207,
                194,
                169,
                119,
                15,
                254,
                110,
                10,
                192,
                165,
                86,
                153,
                15,
                146,
                254,
                228,
                107,
                227,
                90,
                61,
                98
              ]
            }
          }
        },
        {
          "name": "escrow",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  115,
                  99,
                  114,
                  111,
                  119
                ]
              },
              {
                "kind": "account",
                "path": "baseMint"
              }
            ]
          }
        },
        {
          "name": "artistVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  97,
                  114,
                  116,
                  105,
                  115,
                  116,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "baseMint"
              }
            ]
          }
        },
        {
          "name": "unwrap",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  117,
                  110,
                  119,
                  114,
                  97,
                  112
                ]
              },
              {
                "kind": "account",
                "path": "baseMint"
              }
            ]
          }
        },
        {
          "name": "escrowBaseAccount",
          "writable": true
        },
        {
          "name": "charityQuoteAccount",
          "writable": true
        },
        {
          "name": "poolAuthority"
        },
        {
          "name": "pool",
          "writable": true
        },
        {
          "name": "baseVault",
          "writable": true
        },
        {
          "name": "quoteVault",
          "writable": true
        },
        {
          "name": "baseMint"
        },
        {
          "name": "quoteMint",
          "address": "So11111111111111111111111111111111111111112"
        },
        {
          "name": "dbcEventAuthority"
        },
        {
          "name": "dbcProgram",
          "address": "dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN"
        },
        {
          "name": "tokenQuoteProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "tokenBaseProgram",
          "address": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "collectFees",
      "discriminator": [
        164,
        152,
        207,
        99,
        30,
        186,
        19,
        182
      ],
      "accounts": [
        {
          "name": "config",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "coin",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  105,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "baseMint"
              }
            ]
          }
        },
        {
          "name": "escrow",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  115,
                  99,
                  114,
                  111,
                  119
                ]
              },
              {
                "kind": "account",
                "path": "baseMint"
              }
            ]
          }
        },
        {
          "name": "feeAuthority",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  102,
                  101,
                  101,
                  95,
                  97,
                  117,
                  116,
                  104,
                  111,
                  114,
                  105,
                  116,
                  121
                ]
              }
            ]
          }
        },
        {
          "name": "feesVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  102,
                  101,
                  101,
                  115,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              }
            ]
          }
        },
        {
          "name": "artistVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  97,
                  114,
                  116,
                  105,
                  115,
                  116,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "baseMint"
              }
            ]
          }
        },
        {
          "name": "escrowBaseAccount",
          "writable": true
        },
        {
          "name": "launcherQuoteAccount",
          "writable": true
        },
        {
          "name": "treasuryQuoteAccount",
          "writable": true
        },
        {
          "name": "dbcConfig"
        },
        {
          "name": "poolAuthority"
        },
        {
          "name": "pool",
          "writable": true
        },
        {
          "name": "baseVault",
          "writable": true
        },
        {
          "name": "quoteVault",
          "writable": true
        },
        {
          "name": "baseMint"
        },
        {
          "name": "quoteMint",
          "address": "So11111111111111111111111111111111111111112"
        },
        {
          "name": "dbcEventAuthority"
        },
        {
          "name": "dbcProgram",
          "address": "dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN"
        },
        {
          "name": "tokenQuoteProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "tokenBaseProgram",
          "address": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
        }
      ],
      "args": []
    },
    {
      "name": "collectLpFees",
      "discriminator": [
        8,
        174,
        201,
        78,
        141,
        117,
        163,
        33
      ],
      "accounts": [
        {
          "name": "config",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "coin",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  105,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "baseMint"
              }
            ]
          }
        },
        {
          "name": "escrow",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  115,
                  99,
                  114,
                  111,
                  119
                ]
              },
              {
                "kind": "account",
                "path": "baseMint"
              }
            ]
          }
        },
        {
          "name": "feeAuthority",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  102,
                  101,
                  101,
                  95,
                  97,
                  117,
                  116,
                  104,
                  111,
                  114,
                  105,
                  116,
                  121
                ]
              }
            ]
          }
        },
        {
          "name": "feesVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  102,
                  101,
                  101,
                  115,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              }
            ]
          }
        },
        {
          "name": "artistVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  97,
                  114,
                  116,
                  105,
                  115,
                  116,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "baseMint"
              }
            ]
          }
        },
        {
          "name": "escrowBaseAccount",
          "docs": [
            "token A fees, which are zero: the pool collects fees in SOL."
          ],
          "writable": true
        },
        {
          "name": "launcherQuoteAccount",
          "writable": true
        },
        {
          "name": "treasuryQuoteAccount",
          "writable": true
        },
        {
          "name": "dammPoolAuthority"
        },
        {
          "name": "dammPool",
          "writable": true
        },
        {
          "name": "creatorPosition",
          "writable": true
        },
        {
          "name": "creatorPositionNftAccount"
        },
        {
          "name": "partnerPosition",
          "writable": true
        },
        {
          "name": "partnerPositionNftAccount"
        },
        {
          "name": "tokenAVault",
          "writable": true
        },
        {
          "name": "tokenBVault",
          "writable": true
        },
        {
          "name": "baseMint"
        },
        {
          "name": "quoteMint",
          "address": "So11111111111111111111111111111111111111112"
        },
        {
          "name": "dammEventAuthority"
        },
        {
          "name": "dammProgram",
          "address": "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG"
        },
        {
          "name": "tokenQuoteProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "tokenBaseProgram",
          "address": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
        }
      ],
      "args": []
    },
    {
      "name": "createCoin",
      "discriminator": [
        208,
        85,
        34,
        37,
        235,
        182,
        111,
        78
      ],
      "accounts": [
        {
          "name": "launcher",
          "writable": true,
          "signer": true
        },
        {
          "name": "config",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "coin",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  105,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "baseMint"
              }
            ]
          }
        },
        {
          "name": "escrow",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  115,
                  99,
                  114,
                  111,
                  119
                ]
              },
              {
                "kind": "account",
                "path": "baseMint"
              }
            ]
          }
        },
        {
          "name": "artistVault",
          "docs": [
            "Holds the artist's share until they claim."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  97,
                  114,
                  116,
                  105,
                  115,
                  116,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "baseMint"
              }
            ]
          }
        },
        {
          "name": "escrowBaseAccount",
          "docs": [
            "creates the mint. Fee claims need one even though fees come in SOL."
          ],
          "writable": true
        },
        {
          "name": "baseMint",
          "writable": true,
          "signer": true
        },
        {
          "name": "quoteMint",
          "address": "So11111111111111111111111111111111111111112"
        },
        {
          "name": "dbcConfig"
        },
        {
          "name": "poolAuthority"
        },
        {
          "name": "pool",
          "writable": true
        },
        {
          "name": "baseVault",
          "writable": true
        },
        {
          "name": "quoteVault",
          "writable": true
        },
        {
          "name": "dbcEventAuthority"
        },
        {
          "name": "instructionsSysvar",
          "address": "Sysvar1nstructions1111111111111111111111111"
        },
        {
          "name": "dbcProgram",
          "address": "dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN"
        },
        {
          "name": "tokenQuoteProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "tokenProgram",
          "address": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
        },
        {
          "name": "associatedTokenProgram",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "args",
          "type": {
            "defined": {
              "name": "createCoinArgs"
            }
          }
        }
      ]
    },
    {
      "name": "initialize",
      "discriminator": [
        175,
        175,
        109,
        31,
        13,
        152,
        155,
        237
      ],
      "accounts": [
        {
          "name": "admin",
          "docs": [
            "Must be the program's upgrade authority, so nobody can front-run",
            "deployment and claim the config."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "config",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "feeAuthority",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  102,
                  101,
                  101,
                  95,
                  97,
                  117,
                  116,
                  104,
                  111,
                  114,
                  105,
                  116,
                  121
                ]
              }
            ]
          }
        },
        {
          "name": "feesVault",
          "docs": [
            "Where the partner share lands before it's split."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  102,
                  101,
                  101,
                  115,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              }
            ]
          }
        },
        {
          "name": "quoteMint",
          "address": "So11111111111111111111111111111111111111112"
        },
        {
          "name": "program",
          "address": "gATYd3vfHCPCrhFupj8HxVX78syyXNfPSbYRsttm9RF"
        },
        {
          "name": "programData"
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "configParams"
            }
          }
        }
      ]
    },
    {
      "name": "payArtist",
      "discriminator": [
        74,
        123,
        61,
        69,
        64,
        110,
        193,
        151
      ],
      "accounts": [
        {
          "name": "payer",
          "docs": [
            "Pays the rent of `unwrap`, and gets it back."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "coin",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  105,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "baseMint"
              }
            ]
          }
        },
        {
          "name": "passportVault",
          "writable": true
        },
        {
          "name": "escrow",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  115,
                  99,
                  114,
                  111,
                  119
                ]
              },
              {
                "kind": "account",
                "path": "baseMint"
              }
            ]
          }
        },
        {
          "name": "artistVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  97,
                  114,
                  116,
                  105,
                  115,
                  116,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "baseMint"
              }
            ]
          }
        },
        {
          "name": "unwrap",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  117,
                  110,
                  119,
                  114,
                  97,
                  112
                ]
              },
              {
                "kind": "account",
                "path": "baseMint"
              }
            ]
          }
        },
        {
          "name": "baseMint"
        },
        {
          "name": "quoteMint",
          "address": "So11111111111111111111111111111111111111112"
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "sweepCharity",
      "discriminator": [
        136,
        98,
        20,
        158,
        28,
        201,
        113,
        46
      ],
      "accounts": [
        {
          "name": "config",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "coin",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  105,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "baseMint"
              }
            ]
          }
        },
        {
          "name": "escrow",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  115,
                  99,
                  114,
                  111,
                  119
                ]
              },
              {
                "kind": "account",
                "path": "baseMint"
              }
            ]
          }
        },
        {
          "name": "artistVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  97,
                  114,
                  116,
                  105,
                  115,
                  116,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "baseMint"
              }
            ]
          }
        },
        {
          "name": "escrowBaseAccount",
          "writable": true
        },
        {
          "name": "charityQuoteAccount",
          "writable": true
        },
        {
          "name": "poolAuthority"
        },
        {
          "name": "pool",
          "writable": true
        },
        {
          "name": "baseVault",
          "writable": true
        },
        {
          "name": "quoteVault",
          "writable": true
        },
        {
          "name": "baseMint"
        },
        {
          "name": "quoteMint",
          "address": "So11111111111111111111111111111111111111112"
        },
        {
          "name": "dbcEventAuthority"
        },
        {
          "name": "dbcProgram",
          "address": "dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN"
        },
        {
          "name": "tokenQuoteProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "tokenBaseProgram",
          "address": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
        }
      ],
      "args": []
    },
    {
      "name": "updateConfig",
      "discriminator": [
        29,
        158,
        252,
        191,
        10,
        83,
        219,
        99
      ],
      "accounts": [
        {
          "name": "admin",
          "signer": true
        },
        {
          "name": "config",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "configUpdate"
            }
          }
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "coin",
      "discriminator": [
        215,
        195,
        53,
        238,
        217,
        196,
        213,
        51
      ]
    },
    {
      "name": "config",
      "discriminator": [
        155,
        12,
        170,
        224,
        30,
        250,
        204,
        130
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "invalidArtistId",
      "msg": "Artist ID is empty or too long"
    },
    {
      "code": 6001,
      "name": "invalidArtistName",
      "msg": "Artist name is empty or too long"
    },
    {
      "code": 6002,
      "name": "invalidShare",
      "msg": "A share must be at most 100%"
    },
    {
      "code": 6003,
      "name": "invalidClaimWindow",
      "msg": "The claim window must be positive"
    },
    {
      "code": 6004,
      "name": "alreadyClaimed",
      "msg": "This coin's artist has already claimed it"
    },
    {
      "code": 6005,
      "name": "notClaimed",
      "msg": "This coin's artist hasn't claimed it"
    },
    {
      "code": 6006,
      "name": "claimWindowOpen",
      "msg": "The claim window for this coin is still open"
    },
    {
      "code": 6007,
      "name": "wrongPool",
      "msg": "That isn't this coin's pool"
    },
    {
      "code": 6008,
      "name": "wrongReceiver",
      "msg": "That token account belongs to someone else"
    },
    {
      "code": 6009,
      "name": "wrongArtistVault",
      "msg": "That isn't this coin's artist vault"
    },
    {
      "code": 6010,
      "name": "notPassportWallet",
      "msg": "That wallet isn't the passport's"
    },
    {
      "code": 6011,
      "name": "passportRevoked",
      "msg": "The passport is revoked"
    },
    {
      "code": 6012,
      "name": "launchBuyNotSwap",
      "msg": "Only exact-in Meteora swaps can go with a coin's creation"
    },
    {
      "code": 6013,
      "name": "launchBuyTooLarge",
      "msg": "The launch buy is over the cap"
    },
    {
      "code": 6014,
      "name": "mathOverflow",
      "msg": "Math overflow"
    },
    {
      "code": 6015,
      "name": "notUpgradeAuthority",
      "msg": "Only the program's upgrade authority can initialize it"
    }
  ],
  "types": [
    {
      "name": "coin",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "pool",
            "docs": [
              "The coin's Meteora pool."
            ],
            "type": "pubkey"
          },
          {
            "name": "launcher",
            "type": "pubkey"
          },
          {
            "name": "artistId",
            "type": "string"
          },
          {
            "name": "artistName",
            "type": "string"
          },
          {
            "name": "artist",
            "docs": [
              "The artist's passport vault, set once they claim. Their share is paid",
              "there."
            ],
            "type": {
              "option": "pubkey"
            }
          },
          {
            "name": "createdAt",
            "type": "i64"
          },
          {
            "name": "claimDeadline",
            "type": "i64"
          },
          {
            "name": "artistFeesTotal",
            "docs": [
              "Artist fees collected so far (held, paid out, or sent to charity)."
            ],
            "type": "u64"
          },
          {
            "name": "launcherFeesTotal",
            "type": "u64"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "escrowBump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "config",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "admin",
            "type": "pubkey"
          },
          {
            "name": "verifier",
            "docs": [
              "riff's verification service: co-signs artist claims."
            ],
            "type": "pubkey"
          },
          {
            "name": "charity",
            "docs": [
              "Receives artist fees nobody claimed in time."
            ],
            "type": "pubkey"
          },
          {
            "name": "treasury",
            "docs": [
              "Receives riff's part of the partner share."
            ],
            "type": "pubkey"
          },
          {
            "name": "dbcConfig",
            "docs": [
              "The Meteora DBC config every riff coin is created with."
            ],
            "type": "pubkey"
          },
          {
            "name": "launcherShareBps",
            "docs": [
              "Of the partner share, the part for whoever launched the coin (in bps)."
            ],
            "type": "u16"
          },
          {
            "name": "maxLaunchBuy",
            "docs": [
              "The most SOL (in lamports) the launcher's launch buy may spend."
            ],
            "type": "u64"
          },
          {
            "name": "claimWindowSecs",
            "type": "i64"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "feeAuthorityBump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "configParams",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "verifier",
            "type": "pubkey"
          },
          {
            "name": "charity",
            "type": "pubkey"
          },
          {
            "name": "treasury",
            "type": "pubkey"
          },
          {
            "name": "dbcConfig",
            "type": "pubkey"
          },
          {
            "name": "launcherShareBps",
            "type": "u16"
          },
          {
            "name": "maxLaunchBuy",
            "type": "u64"
          },
          {
            "name": "claimWindowSecs",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "configUpdate",
      "docs": [
        "Everything but the Meteora config, which every existing coin depends on.",
        "A new claim window applies to coins created after the change."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "admin",
            "type": "pubkey"
          },
          {
            "name": "verifier",
            "type": "pubkey"
          },
          {
            "name": "charity",
            "type": "pubkey"
          },
          {
            "name": "treasury",
            "type": "pubkey"
          },
          {
            "name": "launcherShareBps",
            "type": "u16"
          },
          {
            "name": "maxLaunchBuy",
            "type": "u64"
          },
          {
            "name": "claimWindowSecs",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "createCoinArgs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "name",
            "type": "string"
          },
          {
            "name": "symbol",
            "type": "string"
          },
          {
            "name": "uri",
            "type": "string"
          },
          {
            "name": "artistId",
            "type": "string"
          },
          {
            "name": "artistName",
            "type": "string"
          }
        ]
      }
    },
    {
      "name": "passport",
      "docs": [
        "An artist's verified identity. Keyed by artist ID, so there's one per artist."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "artistId",
            "type": "string"
          },
          {
            "name": "wallet",
            "type": "pubkey"
          },
          {
            "name": "passkey",
            "type": {
              "array": [
                "u8",
                33
              ]
            }
          },
          {
            "name": "proofs",
            "type": {
              "vec": {
                "defined": {
                  "name": "proofSummary"
                }
              }
            }
          },
          {
            "name": "nonce",
            "docs": [
              "Counts passkey actions; part of every challenge, so no signature works twice."
            ],
            "type": "u64"
          },
          {
            "name": "recovery",
            "type": {
              "option": {
                "defined": {
                  "name": "recovery"
                }
              }
            }
          },
          {
            "name": "revoked",
            "type": "bool"
          },
          {
            "name": "issuedAt",
            "type": "i64"
          },
          {
            "name": "walletSince",
            "docs": [
              "When the current wallet took over; its proof records from before then",
              "can't start a recovery."
            ],
            "type": "i64"
          },
          {
            "name": "withdrawWindowStart",
            "type": "i64"
          },
          {
            "name": "withdrawnInWindow",
            "type": "u64"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "vaultBump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "proofKind",
      "docs": [
        "The ways an artist can prove who they are."
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "spotifyEmail"
          },
          {
            "name": "spotifyProfileCode"
          },
          {
            "name": "website"
          },
          {
            "name": "youTube"
          },
          {
            "name": "instagram"
          },
          {
            "name": "tikTok"
          },
          {
            "name": "x"
          },
          {
            "name": "distributor"
          },
          {
            "name": "appleMusic"
          }
        ]
      }
    },
    {
      "name": "proofSummary",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "kind",
            "type": {
              "defined": {
                "name": "proofKind"
              }
            }
          },
          {
            "name": "sourceHash",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "verifiedAt",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "recovery",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "newWallet",
            "type": "pubkey"
          },
          {
            "name": "newPasskey",
            "type": {
              "array": [
                "u8",
                33
              ]
            }
          },
          {
            "name": "requestedAt",
            "type": "i64"
          },
          {
            "name": "effectiveAt",
            "type": "i64"
          }
        ]
      }
    }
  ]
};
