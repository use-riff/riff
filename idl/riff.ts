/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/riff.json`.
 */
export type Riff = {
  "address": "59MehWKuM1t6u3LAD4nyEg3kBw1HKq5EbS4ByKsosqtV",
  "metadata": {
    "name": "riff",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "Created with Anchor"
  },
  "instructions": [
    {
      "name": "acceptAdmin",
      "discriminator": [
        112,
        42,
        45,
        90,
        116,
        181,
        13,
        170
      ],
      "accounts": [
        {
          "name": "pendingAdmin",
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
      "args": []
    },
    {
      "name": "buy",
      "discriminator": [
        102,
        6,
        61,
        18,
        1,
        218,
        235,
        234
      ],
      "accounts": [
        {
          "name": "trader",
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
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "mint",
          "relations": [
            "coin"
          ]
        },
        {
          "name": "vault",
          "writable": true,
          "relations": [
            "coin"
          ]
        },
        {
          "name": "traderTokenAccount",
          "writable": true
        },
        {
          "name": "tokenProgram",
          "address": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "maxSolIn",
          "type": "u64"
        },
        {
          "name": "minTokensOut",
          "type": "u64"
        }
      ]
    },
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
          "name": "artist",
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
                "path": "coin.mint",
                "account": "coin"
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "artistId",
          "type": "string"
        }
      ]
    },
    {
      "name": "collectProtocolFees",
      "discriminator": [
        22,
        67,
        23,
        98,
        150,
        178,
        70,
        220
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
          "name": "treasury",
          "writable": true
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
                "path": "coin.mint",
                "account": "coin"
              }
            ]
          }
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
          "name": "creator",
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
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "mint",
          "docs": [
            "Fresh keypair. The coin PDA is mint authority only for the length of",
            "this instruction; no freeze authority is ever set."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "vault",
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
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "creatorTokenAccount",
          "docs": [
            "Created only if the creator buys at launch; its address is enforced."
          ],
          "writable": true
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
      "name": "graduate",
      "discriminator": [
        45,
        235,
        225,
        181,
        17,
        218,
        64,
        130
      ],
      "accounts": [
        {
          "name": "payer",
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
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "mint",
          "docs": [
            "Writable only for burning tokens someone planted in the graduation",
            "account (burning lowers the mint's supply)."
          ],
          "writable": true,
          "relations": [
            "coin"
          ]
        },
        {
          "name": "vault",
          "writable": true,
          "relations": [
            "coin"
          ]
        },
        {
          "name": "graduationAuthority",
          "docs": [
            "Signs for riff as the Raydium pool creator. Holds SOL only during",
            "this instruction."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  103,
                  114,
                  97,
                  100,
                  117,
                  97,
                  116,
                  105,
                  111,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "graduationTokenAccount",
          "docs": [
            "emptied and closed here. Address enforced."
          ],
          "writable": true
        },
        {
          "name": "graduationWsolAccount",
          "docs": [
            "and closed here. Address enforced."
          ],
          "writable": true
        },
        {
          "name": "graduationLpAccount",
          "docs": [
            "its LP tokens are burned and it's closed here. Address enforced."
          ],
          "writable": true
        },
        {
          "name": "poolState",
          "docs": [
            "creation. Raydium creates and owns the account."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  111,
                  111,
                  108
                ]
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "raydiumProgram",
          "address": "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C"
        },
        {
          "name": "ammConfig"
        },
        {
          "name": "raydiumAuthority",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  118,
                  97,
                  117,
                  108,
                  116,
                  95,
                  97,
                  110,
                  100,
                  95,
                  108,
                  112,
                  95,
                  109,
                  105,
                  110,
                  116,
                  95,
                  97,
                  117,
                  116,
                  104,
                  95,
                  115,
                  101,
                  101,
                  100
                ]
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                169,
                42,
                90,
                139,
                79,
                41,
                89,
                82,
                132,
                37,
                80,
                170,
                147,
                253,
                91,
                149,
                181,
                172,
                230,
                168,
                235,
                146,
                12,
                147,
                148,
                46,
                67,
                105,
                12,
                32,
                236,
                115
              ]
            }
          }
        },
        {
          "name": "lpMint",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  111,
                  111,
                  108,
                  95,
                  108,
                  112,
                  95,
                  109,
                  105,
                  110,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "poolState"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                169,
                42,
                90,
                139,
                79,
                41,
                89,
                82,
                132,
                37,
                80,
                170,
                147,
                253,
                91,
                149,
                181,
                172,
                230,
                168,
                235,
                146,
                12,
                147,
                148,
                46,
                67,
                105,
                12,
                32,
                236,
                115
              ]
            }
          }
        },
        {
          "name": "poolTokenVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  111,
                  111,
                  108,
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
                "path": "poolState"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                169,
                42,
                90,
                139,
                79,
                41,
                89,
                82,
                132,
                37,
                80,
                170,
                147,
                253,
                91,
                149,
                181,
                172,
                230,
                168,
                235,
                146,
                12,
                147,
                148,
                46,
                67,
                105,
                12,
                32,
                236,
                115
              ]
            }
          }
        },
        {
          "name": "poolWsolVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  111,
                  111,
                  108,
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
                "path": "poolState"
              },
              {
                "kind": "const",
                "value": [
                  6,
                  155,
                  136,
                  87,
                  254,
                  171,
                  129,
                  132,
                  251,
                  104,
                  127,
                  99,
                  70,
                  24,
                  192,
                  53,
                  218,
                  196,
                  57,
                  220,
                  26,
                  235,
                  59,
                  85,
                  152,
                  160,
                  240,
                  0,
                  0,
                  0,
                  0,
                  1
                ]
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                169,
                42,
                90,
                139,
                79,
                41,
                89,
                82,
                132,
                37,
                80,
                170,
                147,
                253,
                91,
                149,
                181,
                172,
                230,
                168,
                235,
                146,
                12,
                147,
                148,
                46,
                67,
                105,
                12,
                32,
                236,
                115
              ]
            }
          }
        },
        {
          "name": "observationState",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  111,
                  98,
                  115,
                  101,
                  114,
                  118,
                  97,
                  116,
                  105,
                  111,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "poolState"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                169,
                42,
                90,
                139,
                79,
                41,
                89,
                82,
                132,
                37,
                80,
                170,
                147,
                253,
                91,
                149,
                181,
                172,
                230,
                168,
                235,
                146,
                12,
                147,
                148,
                46,
                67,
                105,
                12,
                32,
                236,
                115
              ]
            }
          }
        },
        {
          "name": "createPoolFeeReceiver",
          "writable": true,
          "address": "DNXgeM9EiiaAbaWvwjHj9fQQLAX5ZsfHyvmYUNRAdNC8"
        },
        {
          "name": "wsolMint",
          "address": "So11111111111111111111111111111111111111112"
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "token2022Program",
          "address": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
        },
        {
          "name": "associatedTokenProgram",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        },
        {
          "name": "rent",
          "address": "SysvarRent111111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "initializeConfig",
      "discriminator": [
        208,
        127,
        21,
        1,
        194,
        190,
        196,
        70
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
          "name": "program",
          "address": "59MehWKuM1t6u3LAD4nyEg3kBw1HKq5EbS4ByKsosqtV"
        },
        {
          "name": "programData"
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
      "name": "prepareGraduation",
      "discriminator": [
        69,
        188,
        237,
        109,
        88,
        218,
        159,
        115
      ],
      "accounts": [
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
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "mint",
          "relations": [
            "coin"
          ]
        },
        {
          "name": "graduationAuthority",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  103,
                  114,
                  97,
                  100,
                  117,
                  97,
                  116,
                  105,
                  111,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ]
          }
        }
      ],
      "args": []
    },
    {
      "name": "sell",
      "discriminator": [
        51,
        230,
        133,
        164,
        1,
        127,
        131,
        173
      ],
      "accounts": [
        {
          "name": "trader",
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
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "mint",
          "relations": [
            "coin"
          ]
        },
        {
          "name": "vault",
          "writable": true,
          "relations": [
            "coin"
          ]
        },
        {
          "name": "traderTokenAccount",
          "writable": true
        },
        {
          "name": "tokenProgram",
          "address": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "tokenAmount",
          "type": "u64"
        },
        {
          "name": "minSolOut",
          "type": "u64"
        }
      ]
    },
    {
      "name": "sweepCharityFees",
      "discriminator": [
        116,
        200,
        208,
        102,
        215,
        44,
        242,
        66
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
          "name": "charity",
          "writable": true
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
                "path": "coin.mint",
                "account": "coin"
              }
            ]
          }
        }
      ],
      "args": []
    },
    {
      "name": "transferAdmin",
      "discriminator": [
        42,
        242,
        66,
        106,
        228,
        10,
        111,
        156
      ],
      "accounts": [
        {
          "name": "admin",
          "signer": true,
          "relations": [
            "config"
          ]
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
          "name": "newAdmin",
          "type": "pubkey"
        }
      ]
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
          "signer": true,
          "relations": [
            "config"
          ]
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
              "name": "updateConfigParams"
            }
          }
        }
      ]
    },
    {
      "name": "withdrawArtistFees",
      "discriminator": [
        204,
        60,
        30,
        111,
        252,
        140,
        194,
        28
      ],
      "accounts": [
        {
          "name": "artist",
          "writable": true,
          "signer": true
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
                "path": "coin.mint",
                "account": "coin"
              }
            ]
          }
        }
      ],
      "args": []
    },
    {
      "name": "withdrawCreatorFees",
      "discriminator": [
        8,
        30,
        213,
        18,
        121,
        105,
        129,
        222
      ],
      "accounts": [
        {
          "name": "creator",
          "writable": true,
          "signer": true,
          "relations": [
            "coin"
          ]
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
                "path": "coin.mint",
                "account": "coin"
              }
            ]
          }
        }
      ],
      "args": []
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
  "events": [
    {
      "name": "adminTransferProposed",
      "discriminator": [
        203,
        168,
        175,
        51,
        239,
        104,
        20,
        85
      ]
    },
    {
      "name": "adminTransferred",
      "discriminator": [
        255,
        147,
        182,
        5,
        199,
        217,
        38,
        179
      ]
    },
    {
      "name": "artistClaimed",
      "discriminator": [
        49,
        29,
        178,
        17,
        116,
        114,
        250,
        30
      ]
    },
    {
      "name": "artistFeesWithdrawn",
      "discriminator": [
        236,
        247,
        32,
        66,
        72,
        22,
        78,
        251
      ]
    },
    {
      "name": "charityFeesSwept",
      "discriminator": [
        187,
        49,
        44,
        93,
        115,
        39,
        75,
        51
      ]
    },
    {
      "name": "coinCreated",
      "discriminator": [
        155,
        248,
        204,
        17,
        205,
        237,
        158,
        104
      ]
    },
    {
      "name": "configInitialized",
      "discriminator": [
        181,
        49,
        200,
        156,
        19,
        167,
        178,
        91
      ]
    },
    {
      "name": "configUpdated",
      "discriminator": [
        40,
        241,
        230,
        122,
        11,
        19,
        198,
        194
      ]
    },
    {
      "name": "creatorFeesWithdrawn",
      "discriminator": [
        142,
        52,
        192,
        191,
        6,
        90,
        253,
        62
      ]
    },
    {
      "name": "curveCompleted",
      "discriminator": [
        1,
        174,
        164,
        127,
        219,
        129,
        243,
        14
      ]
    },
    {
      "name": "graduated",
      "discriminator": [
        51,
        241,
        66,
        50,
        140,
        245,
        156,
        192
      ]
    },
    {
      "name": "graduationPrepared",
      "discriminator": [
        12,
        183,
        149,
        64,
        194,
        44,
        234,
        227
      ]
    },
    {
      "name": "protocolFeesCollected",
      "discriminator": [
        165,
        34,
        125,
        155,
        15,
        86,
        99,
        191
      ]
    },
    {
      "name": "trade",
      "discriminator": [
        24,
        254,
        218,
        152,
        253,
        43,
        18,
        81
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "notUpgradeAuthority",
      "msg": "Signer is not the program upgrade authority"
    },
    {
      "code": 6001,
      "name": "invalidClaimWindow",
      "msg": "Claim window must be greater than zero"
    },
    {
      "code": 6002,
      "name": "invalidTradeFee",
      "msg": "Total trading fee must be at most 1000 bps"
    },
    {
      "code": 6003,
      "name": "invalidCreatorBuyCap",
      "msg": "Creator buy cap must be at most 1000 bps of supply"
    },
    {
      "code": 6004,
      "name": "invalidCurveParams",
      "msg": "Invalid bonding curve parameters"
    },
    {
      "code": 6005,
      "name": "invalidName",
      "msg": "Token name must be 1-32 bytes"
    },
    {
      "code": 6006,
      "name": "invalidSymbol",
      "msg": "Token symbol must be 1-10 bytes"
    },
    {
      "code": 6007,
      "name": "invalidUri",
      "msg": "Metadata URI must be 1-200 bytes"
    },
    {
      "code": 6008,
      "name": "invalidArtistId",
      "msg": "Artist ID must be 1-64 printable ASCII characters with no spaces"
    },
    {
      "code": 6009,
      "name": "invalidArtistName",
      "msg": "Artist name must be 1-64 bytes"
    },
    {
      "code": 6010,
      "name": "mathOverflow",
      "msg": "Arithmetic overflow"
    },
    {
      "code": 6011,
      "name": "amountTooSmall",
      "msg": "Amount is zero or too small to trade"
    },
    {
      "code": 6012,
      "name": "slippageExceeded",
      "msg": "Price moved beyond the slippage limit"
    },
    {
      "code": 6013,
      "name": "curveComplete",
      "msg": "Bonding curve is complete; trading is closed until graduation"
    },
    {
      "code": 6014,
      "name": "creatorBuyTooLarge",
      "msg": "Creator launch buy exceeds the cap"
    },
    {
      "code": 6015,
      "name": "tradingNotOpen",
      "msg": "Trading opens the slot after launch"
    },
    {
      "code": 6016,
      "name": "noFeesToWithdraw",
      "msg": "No fees to withdraw"
    },
    {
      "code": 6017,
      "name": "treasuryNotRentExempt",
      "msg": "Collection would leave the treasury below the rent-exempt minimum"
    },
    {
      "code": 6018,
      "name": "coinUnderfunded",
      "msg": "Payout would leave the coin account unable to cover what it owes"
    },
    {
      "code": 6019,
      "name": "charityNotRentExempt",
      "msg": "Sweep would leave the charity wallet below the rent-exempt minimum"
    },
    {
      "code": 6020,
      "name": "invalidPayoutRecipient",
      "msg": "A coin can't pay out to itself"
    },
    {
      "code": 6021,
      "name": "invalidAddress",
      "msg": "Address must not be the default (all zeros) key"
    },
    {
      "code": 6022,
      "name": "notVerifier",
      "msg": "Claim must be co-signed by the configured verifier"
    },
    {
      "code": 6023,
      "name": "alreadyClaimed",
      "msg": "This coin's artist has already claimed it"
    },
    {
      "code": 6024,
      "name": "artistIdMismatch",
      "msg": "Artist ID doesn't match the coin's artist"
    },
    {
      "code": 6025,
      "name": "notArtist",
      "msg": "Only the coin's claimed artist can do this"
    },
    {
      "code": 6026,
      "name": "notAdmin",
      "msg": "Only the config admin can do this"
    },
    {
      "code": 6027,
      "name": "notPendingAdmin",
      "msg": "Only the proposed admin can accept"
    },
    {
      "code": 6028,
      "name": "curveNotComplete",
      "msg": "The curve hasn't sold out yet"
    },
    {
      "code": 6029,
      "name": "alreadyGraduated",
      "msg": "This coin has already graduated"
    },
    {
      "code": 6030,
      "name": "reserveMissing",
      "msg": "The vault holds less than the graduation reserve"
    },
    {
      "code": 6031,
      "name": "invalidAmmConfig",
      "msg": "Account isn't the configured Raydium fee tier"
    },
    {
      "code": 6032,
      "name": "graduationNotPrepared",
      "msg": "Graduation needs prepare_graduation first"
    },
    {
      "code": 6033,
      "name": "graduationAlreadyPrepared",
      "msg": "Graduation is already prepared"
    }
  ],
  "types": [
    {
      "name": "adminTransferProposed",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "admin",
            "type": "pubkey"
          },
          {
            "name": "pendingAdmin",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "adminTransferred",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "previousAdmin",
            "type": "pubkey"
          },
          {
            "name": "admin",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "artistClaimed",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "coin",
            "type": "pubkey"
          },
          {
            "name": "artist",
            "type": "pubkey"
          },
          {
            "name": "artistId",
            "type": "string"
          },
          {
            "name": "late",
            "docs": [
              "Claimed after the claim window closed."
            ],
            "type": "bool"
          },
          {
            "name": "forfeitedToCharity",
            "docs": [
              "Fees held for the artist that went to the charity instead because",
              "the claim was late."
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "artistFeesWithdrawn",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "coin",
            "type": "pubkey"
          },
          {
            "name": "artist",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "charityFeesSwept",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "coin",
            "type": "pubkey"
          },
          {
            "name": "charity",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "coin",
      "docs": [
        "One launched coin. PDA at `[COIN_SEED, mint]`.",
        "",
        "The artist is recorded only as text until they claim the coin; until then",
        "`artist` is `None` and nothing implies the artist is affiliated with it.",
        "",
        "Holds the curve's SOL and all uncollected fees as lamports, on top of its",
        "own rent."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "vault",
            "docs": [
              "Program-owned token account holding the full supply."
            ],
            "type": "pubkey"
          },
          {
            "name": "creator",
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
              "Set when the artist claims the coin."
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
            "name": "createdSlot",
            "docs": [
              "Slot the coin was created in. `buy` is closed for this slot, so the",
              "creator's capped launch buy is the only purchase possible at launch."
            ],
            "type": "u64"
          },
          {
            "name": "claimDeadline",
            "docs": [
              "Last moment the artist may claim, fixed at creation from the config's",
              "claim window so later config changes don't move it."
            ],
            "type": "i64"
          },
          {
            "name": "virtualSolReserves",
            "type": "u64"
          },
          {
            "name": "virtualTokenReserves",
            "type": "u64"
          },
          {
            "name": "realSolReserves",
            "type": "u64"
          },
          {
            "name": "realTokenReserves",
            "type": "u64"
          },
          {
            "name": "artistFees",
            "docs": [
              "Artist fees held for the artist, in lamports: accrued before the claim",
              "window closed, or after the artist claimed."
            ],
            "type": "u64"
          },
          {
            "name": "charityFees",
            "docs": [
              "Artist-share fees owed to the charity and not yet swept, in lamports."
            ],
            "type": "u64"
          },
          {
            "name": "creatorFees",
            "docs": [
              "Creator fees accrued and not yet withdrawn, in lamports."
            ],
            "type": "u64"
          },
          {
            "name": "protocolFees",
            "docs": [
              "Protocol fees accrued and not yet collected to the treasury, in lamports."
            ],
            "type": "u64"
          },
          {
            "name": "complete",
            "docs": [
              "Every curve token has been sold; trading stops until graduation."
            ],
            "type": "bool"
          },
          {
            "name": "pool",
            "docs": [
              "Raydium pool the coin graduated to. Set once, by `graduate`."
            ],
            "type": {
              "option": "pubkey"
            }
          },
          {
            "name": "graduationSol",
            "docs": [
              "Curve SOL handed to the graduation authority by `prepare_graduation`",
              "and not yet deposited in the pool (0 before and after graduation)."
            ],
            "type": "u64"
          },
          {
            "name": "artistFeesTotal",
            "docs": [
              "Every artist share of every curve trade fee, in lamports, whether it",
              "went to the artist or to charity. Never decreases."
            ],
            "type": "u64"
          },
          {
            "name": "peakVirtualSol",
            "docs": [
              "Highest virtual SOL reserves the curve has reached. The price only",
              "rises with virtual SOL, so this marks the curve's all-time-high price."
            ],
            "type": "u64"
          },
          {
            "name": "volumeHourly",
            "docs": [
              "Curve trade volume (SOL into or out of the curve, before fees) per",
              "hour for the last 24 hours: `volume_hourly[h % 24]` holds unix hour",
              "`h`, for `h` in `volume_hour - 23 ..= volume_hour`."
            ],
            "type": {
              "array": [
                "u64",
                24
              ]
            }
          },
          {
            "name": "volumeHour",
            "docs": [
              "Unix hour (seconds / 3600) of the latest trade."
            ],
            "type": "i64"
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
      "name": "coinCreated",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "coin",
            "type": "pubkey"
          },
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "creator",
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
            "name": "claimDeadline",
            "type": "i64"
          },
          {
            "name": "creatorBuyTokens",
            "docs": [
              "Tokens the creator bought at launch (0 if none)."
            ],
            "type": "u64"
          },
          {
            "name": "creatorBuySol",
            "docs": [
              "SOL the creator paid for them, fees included."
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "config",
      "docs": [
        "Protocol-wide settings. Singleton PDA at `[CONFIG_SEED]`."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "admin",
            "type": "pubkey"
          },
          {
            "name": "pendingAdmin",
            "docs": [
              "Proposed next admin; becomes admin only by signing `accept_admin`."
            ],
            "type": {
              "option": "pubkey"
            }
          },
          {
            "name": "treasury",
            "docs": [
              "Receives the protocol's share of trading fees, in batches, via the",
              "permissionless `collect_protocol_fees`. Trades never touch it, so it",
              "needn't sign or exist in advance (e.g. a Squads vault)."
            ],
            "type": "pubkey"
          },
          {
            "name": "charity",
            "docs": [
              "Receives artist fees nobody can claim: the artist's share for a coin",
              "whose claim window closed unclaimed. riff donates from it."
            ],
            "type": "pubkey"
          },
          {
            "name": "verifier",
            "docs": [
              "Co-signs every `claim_artist`, vouching that the claiming wallet",
              "belongs to the coin's artist. Held by riff's verification service."
            ],
            "type": "pubkey"
          },
          {
            "name": "raydiumAmmConfig",
            "docs": [
              "Raydium CPMM fee tier (`AmmConfig` account) graduated coins' pools use."
            ],
            "type": "pubkey"
          },
          {
            "name": "artistFeeBps",
            "docs": [
              "Trading fee rates, each in basis points of a trade's SOL amount. Their",
              "sum is the total fee charged."
            ],
            "type": "u16"
          },
          {
            "name": "creatorFeeBps",
            "type": "u16"
          },
          {
            "name": "protocolFeeBps",
            "type": "u16"
          },
          {
            "name": "claimWindowSecs",
            "docs": [
              "How long after a coin's creation its artist may claim it, in seconds."
            ],
            "type": "i64"
          },
          {
            "name": "initialVirtualSolReserves",
            "docs": [
              "Starting virtual reserves for every new coin's curve."
            ],
            "type": "u64"
          },
          {
            "name": "initialVirtualTokenReserves",
            "docs": [
              "Derived from the supply split so the graduation pool opens at the",
              "curve's final price; see `curve::graduation_virtual_token_reserves`."
            ],
            "type": "u64"
          },
          {
            "name": "curveTokenSupply",
            "docs": [
              "Tokens sellable on the curve; the rest of the supply stays in the",
              "vault as the graduation reserve."
            ],
            "type": "u64"
          },
          {
            "name": "maxCreatorBuyBps",
            "docs": [
              "Most the creator may buy at launch, in basis points of total supply."
            ],
            "type": "u16"
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "configInitialized",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "admin",
            "type": "pubkey"
          },
          {
            "name": "treasury",
            "type": "pubkey"
          },
          {
            "name": "charity",
            "type": "pubkey"
          },
          {
            "name": "verifier",
            "type": "pubkey"
          },
          {
            "name": "raydiumAmmConfig",
            "type": "pubkey"
          },
          {
            "name": "artistFeeBps",
            "type": "u16"
          },
          {
            "name": "creatorFeeBps",
            "type": "u16"
          },
          {
            "name": "protocolFeeBps",
            "type": "u16"
          },
          {
            "name": "claimWindowSecs",
            "type": "i64"
          },
          {
            "name": "initialVirtualSolReserves",
            "type": "u64"
          },
          {
            "name": "initialVirtualTokenReserves",
            "type": "u64"
          },
          {
            "name": "curveTokenSupply",
            "type": "u64"
          },
          {
            "name": "maxCreatorBuyBps",
            "type": "u16"
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
            "name": "treasury",
            "type": "pubkey"
          },
          {
            "name": "charity",
            "type": "pubkey"
          },
          {
            "name": "verifier",
            "type": "pubkey"
          },
          {
            "name": "raydiumAmmConfig",
            "type": "pubkey"
          },
          {
            "name": "artistFeeBps",
            "docs": [
              "Fee rates in basis points of each trade's SOL amount."
            ],
            "type": "u16"
          },
          {
            "name": "creatorFeeBps",
            "type": "u16"
          },
          {
            "name": "protocolFeeBps",
            "type": "u16"
          },
          {
            "name": "claimWindowSecs",
            "type": "i64"
          },
          {
            "name": "initialVirtualSolReserves",
            "type": "u64"
          },
          {
            "name": "curveTokenSupply",
            "docs": [
              "Tokens sold on the curve. The graduation reserve is the rest of the",
              "supply, and the starting virtual token reserves are derived from both."
            ],
            "type": "u64"
          },
          {
            "name": "maxCreatorBuyBps",
            "docs": [
              "Creator launch-buy cap, in basis points of total supply."
            ],
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "configUpdated",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "treasury",
            "type": "pubkey"
          },
          {
            "name": "charity",
            "type": "pubkey"
          },
          {
            "name": "verifier",
            "type": "pubkey"
          },
          {
            "name": "raydiumAmmConfig",
            "type": "pubkey"
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
            "docs": [
              "External artist identifier (e.g. a streaming-platform artist ID)."
            ],
            "type": "string"
          },
          {
            "name": "artistName",
            "type": "string"
          },
          {
            "name": "creatorBuySol",
            "docs": [
              "Most SOL (fee included) the creator spends buying at launch, from the",
              "same curve as everyone else. 0 for no buy. The tokens bought may not",
              "exceed the config's cap."
            ],
            "type": "u64"
          },
          {
            "name": "creatorBuyMinTokens",
            "docs": [
              "Fewest tokens the launch buy may return. The curve is fresh, but the",
              "config's fees and curve settings could change between signing and",
              "landing."
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "creatorFeesWithdrawn",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "coin",
            "type": "pubkey"
          },
          {
            "name": "creator",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "curveCompleted",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "coin",
            "type": "pubkey"
          },
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "realSolReserves",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "graduated",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "coin",
            "type": "pubkey"
          },
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "solAmount",
            "docs": [
              "SOL deposited into the pool: exactly what the curve raised."
            ],
            "type": "u64"
          },
          {
            "name": "tokenAmount",
            "docs": [
              "Tokens deposited: exactly the graduation reserve."
            ],
            "type": "u64"
          },
          {
            "name": "lpBurned",
            "docs": [
              "Every LP token minted to riff, burned so the liquidity is locked."
            ],
            "type": "u64"
          },
          {
            "name": "costReimbursed",
            "docs": [
              "Pool-creation cost reimbursed to the caller from protocol fees."
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "graduationPrepared",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "coin",
            "type": "pubkey"
          },
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "solAmount",
            "docs": [
              "Curve SOL moved to the graduation authority for the pool."
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "protocolFeesCollected",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "coin",
            "type": "pubkey"
          },
          {
            "name": "treasury",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "trade",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "coin",
            "type": "pubkey"
          },
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "trader",
            "type": "pubkey"
          },
          {
            "name": "isBuy",
            "type": "bool"
          },
          {
            "name": "solAmount",
            "docs": [
              "SOL into (buy) or out of (sell) the curve's reserves, before fees."
            ],
            "type": "u64"
          },
          {
            "name": "tokenAmount",
            "type": "u64"
          },
          {
            "name": "artistFee",
            "type": "u64"
          },
          {
            "name": "creatorFee",
            "type": "u64"
          },
          {
            "name": "protocolFee",
            "type": "u64"
          },
          {
            "name": "virtualSolReserves",
            "type": "u64"
          },
          {
            "name": "virtualTokenReserves",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "updateConfigParams",
      "docs": [
        "Settings to change; `None` leaves one unchanged. Fees and curve settings",
        "can't be changed here, and nothing here can touch funds held by coins."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "treasury",
            "type": {
              "option": "pubkey"
            }
          },
          {
            "name": "charity",
            "type": {
              "option": "pubkey"
            }
          },
          {
            "name": "verifier",
            "type": {
              "option": "pubkey"
            }
          },
          {
            "name": "raydiumAmmConfig",
            "type": {
              "option": "pubkey"
            }
          },
          {
            "name": "claimWindowSecs",
            "docs": [
              "Claim window for coins launched from now on. Existing coins keep the",
              "deadline they got at launch."
            ],
            "type": {
              "option": "i64"
            }
          }
        ]
      }
    }
  ],
  "constants": [
    {
      "name": "bpsDenominator",
      "docs": [
        "Basis-point denominator (100%)."
      ],
      "type": "u16",
      "value": "10000"
    },
    {
      "name": "coinDecimals",
      "type": "u8",
      "value": "6"
    },
    {
      "name": "coinSeed",
      "type": "bytes",
      "value": "[99, 111, 105, 110]"
    },
    {
      "name": "coinTotalSupply",
      "docs": [
        "1,000,000,000 whole tokens, in base units. Minted once at creation; never more."
      ],
      "type": "u64",
      "value": "1000000000000000"
    },
    {
      "name": "configSeed",
      "type": "bytes",
      "value": "[99, 111, 110, 102, 105, 103]"
    },
    {
      "name": "graduationSeed",
      "type": "bytes",
      "value": "[103, 114, 97, 100, 117, 97, 116, 105, 111, 110]"
    },
    {
      "name": "maxCreatorBuyBps",
      "docs": [
        "Highest creator launch-buy cap the config will accept (10% of supply)."
      ],
      "type": "u16",
      "value": "1000"
    },
    {
      "name": "maxTradeFeeBps",
      "docs": [
        "Highest total trading fee the config will accept (10%)."
      ],
      "type": "u16",
      "value": "1000"
    },
    {
      "name": "poolSeed",
      "docs": [
        "riff PDA used as the graduated coin's Raydium pool address. Only riff can",
        "sign for it, so nobody can create the graduation pool first."
      ],
      "type": "bytes",
      "value": "[112, 111, 111, 108]"
    },
    {
      "name": "vaultSeed",
      "type": "bytes",
      "value": "[118, 97, 117, 108, 116]"
    }
  ]
};
