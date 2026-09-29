/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/riff_passport.json`.
 */
export type RiffPassport = {
  "address": "2nke6euXvAnbtdtcbbk8N2Z3SRLuR7i67VYmSdcY5kwj",
  "metadata": {
    "name": "riffPassport",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "Artist Passport: a multi-proof, passkey-protected on-chain identity for musicians"
  },
  "instructions": [
    {
      "name": "addProofs",
      "discriminator": [
        68,
        239,
        81,
        242,
        64,
        61,
        148,
        159
      ],
      "accounts": [
        {
          "name": "wallet",
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
          "name": "passport",
          "writable": true,
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
                "path": "passport.artistId",
                "account": "passport"
              }
            ]
          }
        },
        {
          "name": "instructions",
          "address": "Sysvar1nstructions1111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "passkeyProof",
          "type": {
            "defined": {
              "name": "passkeyProof"
            }
          }
        }
      ]
    },
    {
      "name": "claimCoin",
      "discriminator": [
        222,
        156,
        154,
        212,
        188,
        143,
        84,
        105
      ],
      "accounts": [
        {
          "name": "wallet",
          "signer": true
        },
        {
          "name": "verifier",
          "docs": [
            "riff's verifier, co-signing as riff's `claim_artist` requires."
          ],
          "signer": true
        },
        {
          "name": "passport",
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
                "path": "passport.artistId",
                "account": "passport"
              }
            ]
          }
        },
        {
          "name": "vault",
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
            ]
          }
        },
        {
          "name": "riffConfig"
        },
        {
          "name": "coin",
          "writable": true
        },
        {
          "name": "riffProgram",
          "address": "59MehWKuM1t6u3LAD4nyEg3kBw1HKq5EbS4ByKsosqtV"
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
          "name": "passport",
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
                "path": "passport.artistId",
                "account": "passport"
              }
            ]
          }
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
                "path": "passport"
              }
            ]
          }
        },
        {
          "name": "coin",
          "writable": true
        },
        {
          "name": "riffProgram",
          "address": "59MehWKuM1t6u3LAD4nyEg3kBw1HKq5EbS4ByKsosqtV"
        }
      ],
      "args": []
    },
    {
      "name": "disavow",
      "discriminator": [
        175,
        27,
        245,
        121,
        155,
        144,
        152,
        23
      ],
      "accounts": [
        {
          "name": "wallet",
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
          "name": "passport",
          "writable": true,
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
                "path": "passport.artistId",
                "account": "passport"
              }
            ]
          }
        },
        {
          "name": "endorsement",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  110,
                  100,
                  111,
                  114,
                  115,
                  101
                ]
              },
              {
                "kind": "account",
                "path": "passport"
              },
              {
                "kind": "arg",
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "instructions",
          "address": "Sysvar1nstructions1111111111111111111111111"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "mint",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "endorse",
      "discriminator": [
        2,
        228,
        252,
        182,
        105,
        92,
        40,
        175
      ],
      "accounts": [
        {
          "name": "wallet",
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
          "name": "passport",
          "writable": true,
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
                "path": "passport.artistId",
                "account": "passport"
              }
            ]
          }
        },
        {
          "name": "endorsement",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  110,
                  100,
                  111,
                  114,
                  115,
                  101
                ]
              },
              {
                "kind": "account",
                "path": "passport"
              },
              {
                "kind": "arg",
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "instructions",
          "address": "Sysvar1nstructions1111111111111111111111111"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "mint",
          "type": "pubkey"
        },
        {
          "name": "passkeyProof",
          "type": {
            "defined": {
              "name": "passkeyProof"
            }
          }
        }
      ]
    },
    {
      "name": "finalizeRecovery",
      "discriminator": [
        180,
        175,
        56,
        254,
        138,
        101,
        151,
        219
      ],
      "accounts": [
        {
          "name": "passport",
          "writable": true,
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
                "path": "passport.artistId",
                "account": "passport"
              }
            ]
          }
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
            "Must be the program's upgrade authority, so nobody can front-run the config."
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
          "address": "2nke6euXvAnbtdtcbbk8N2Z3SRLuR7i67VYmSdcY5kwj"
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
              "name": "passportConfigParams"
            }
          }
        }
      ]
    },
    {
      "name": "issuePassport",
      "discriminator": [
        143,
        170,
        86,
        204,
        142,
        226,
        52,
        80
      ],
      "accounts": [
        {
          "name": "wallet",
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
          "name": "passport",
          "writable": true,
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
                "kind": "arg",
                "path": "artistId"
              }
            ]
          }
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
                "path": "passport"
              }
            ]
          }
        },
        {
          "name": "instructions",
          "address": "Sysvar1nstructions1111111111111111111111111"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "artistId",
          "type": "string"
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
          "name": "passkeyProof",
          "type": {
            "defined": {
              "name": "passkeyProof"
            }
          }
        }
      ]
    },
    {
      "name": "recordProof",
      "discriminator": [
        144,
        172,
        144,
        35,
        124,
        170,
        93,
        80
      ],
      "accounts": [
        {
          "name": "wallet",
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
          "name": "proofRecord",
          "writable": true
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "artistId",
          "type": "string"
        },
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
        }
      ]
    },
    {
      "name": "requestRecovery",
      "discriminator": [
        169,
        44,
        164,
        157,
        148,
        175,
        110,
        130
      ],
      "accounts": [
        {
          "name": "newWallet",
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
          "name": "passport",
          "writable": true,
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
                "path": "passport.artistId",
                "account": "passport"
              }
            ]
          }
        },
        {
          "name": "instructions",
          "address": "Sysvar1nstructions1111111111111111111111111"
        }
      ],
      "args": [
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
          "name": "passkeyProof",
          "type": {
            "defined": {
              "name": "passkeyProof"
            }
          }
        }
      ]
    },
    {
      "name": "revokePassport",
      "discriminator": [
        183,
        230,
        237,
        155,
        96,
        111,
        120,
        13
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
          "name": "passport",
          "writable": true,
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
                "path": "passport.artistId",
                "account": "passport"
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "reason",
          "type": "string"
        }
      ]
    },
    {
      "name": "setPasskey",
      "discriminator": [
        144,
        207,
        13,
        132,
        79,
        238,
        151,
        142
      ],
      "accounts": [
        {
          "name": "wallet",
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
          "name": "passport",
          "writable": true,
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
                "path": "passport.artistId",
                "account": "passport"
              }
            ]
          }
        },
        {
          "name": "instructions",
          "address": "Sysvar1nstructions1111111111111111111111111"
        }
      ],
      "args": [
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
          "name": "oldProof",
          "type": {
            "defined": {
              "name": "passkeyProof"
            }
          }
        },
        {
          "name": "newProof",
          "type": {
            "defined": {
              "name": "passkeyProof"
            }
          }
        }
      ]
    },
    {
      "name": "setWallet",
      "discriminator": [
        160,
        158,
        130,
        75,
        194,
        97,
        39,
        114
      ],
      "accounts": [
        {
          "name": "act",
          "accounts": [
            {
              "name": "wallet",
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
              "name": "passport",
              "writable": true,
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
                    "path": "passport.artistId",
                    "account": "passport"
                  }
                ]
              }
            },
            {
              "name": "instructions",
              "address": "Sysvar1nstructions1111111111111111111111111"
            }
          ]
        },
        {
          "name": "newWallet",
          "docs": [
            "Signs too, so a typo can't send the passport to a wallet nobody holds."
          ],
          "signer": true
        }
      ],
      "args": [
        {
          "name": "passkeyProof",
          "type": {
            "defined": {
              "name": "passkeyProof"
            }
          }
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
              "name": "passportConfigParams"
            }
          }
        }
      ]
    },
    {
      "name": "vetoRecovery",
      "discriminator": [
        57,
        30,
        97,
        87,
        158,
        139,
        31,
        7
      ],
      "accounts": [
        {
          "name": "vetoer",
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
          "name": "passport",
          "writable": true,
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
                "path": "passport.artistId",
                "account": "passport"
              }
            ]
          }
        },
        {
          "name": "instructions",
          "address": "Sysvar1nstructions1111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "passkeyProof",
          "type": {
            "option": {
              "defined": {
                "name": "passkeyProof"
              }
            }
          }
        }
      ]
    },
    {
      "name": "withdraw",
      "discriminator": [
        183,
        18,
        70,
        156,
        148,
        109,
        161,
        34
      ],
      "accounts": [
        {
          "name": "wallet",
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
          "name": "passport",
          "writable": true,
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
                "path": "passport.artistId",
                "account": "passport"
              }
            ]
          }
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
                "path": "passport"
              }
            ]
          }
        },
        {
          "name": "instructions",
          "address": "Sysvar1nstructions1111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        },
        {
          "name": "passkeyProof",
          "type": {
            "option": {
              "defined": {
                "name": "passkeyProof"
              }
            }
          }
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "endorsement",
      "discriminator": [
        167,
        137,
        37,
        17,
        220,
        102,
        104,
        52
      ]
    },
    {
      "name": "passport",
      "discriminator": [
        18,
        61,
        245,
        239,
        6,
        15,
        18,
        34
      ]
    },
    {
      "name": "passportConfig",
      "discriminator": [
        247,
        219,
        200,
        32,
        96,
        189,
        165,
        250
      ]
    },
    {
      "name": "proofRecord",
      "discriminator": [
        237,
        59,
        155,
        172,
        204,
        117,
        87,
        44
      ]
    },
    {
      "name": "vault",
      "discriminator": [
        211,
        8,
        232,
        43,
        2,
        152,
        117,
        119
      ]
    }
  ],
  "events": [
    {
      "name": "coinClaimed",
      "discriminator": [
        110,
        254,
        3,
        227,
        117,
        77,
        164,
        24
      ]
    },
    {
      "name": "endorsementChanged",
      "discriminator": [
        125,
        49,
        202,
        166,
        43,
        96,
        251,
        32
      ]
    },
    {
      "name": "passkeyChanged",
      "discriminator": [
        246,
        125,
        218,
        44,
        88,
        68,
        127,
        72
      ]
    },
    {
      "name": "passportIssued",
      "discriminator": [
        53,
        32,
        67,
        238,
        156,
        237,
        250,
        225
      ]
    },
    {
      "name": "passportRevoked",
      "discriminator": [
        204,
        235,
        22,
        2,
        134,
        110,
        206,
        6
      ]
    },
    {
      "name": "proofRecorded",
      "discriminator": [
        55,
        159,
        128,
        127,
        158,
        254,
        93,
        145
      ]
    },
    {
      "name": "proofsAdded",
      "discriminator": [
        30,
        143,
        239,
        243,
        0,
        176,
        248,
        208
      ]
    },
    {
      "name": "recoveryFinalized",
      "discriminator": [
        197,
        135,
        228,
        62,
        219,
        96,
        110,
        145
      ]
    },
    {
      "name": "recoveryRequested",
      "discriminator": [
        127,
        3,
        38,
        230,
        145,
        28,
        53,
        141
      ]
    },
    {
      "name": "recoveryVetoed",
      "discriminator": [
        214,
        202,
        88,
        71,
        245,
        187,
        9,
        87
      ]
    },
    {
      "name": "vaultWithdrawn",
      "discriminator": [
        238,
        9,
        219,
        172,
        188,
        77,
        72,
        104
      ]
    },
    {
      "name": "walletChanged",
      "discriminator": [
        190,
        6,
        164,
        64,
        90,
        85,
        216,
        206
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
      "name": "notAdmin",
      "msg": "Signer is not the passport config admin"
    },
    {
      "code": 6002,
      "name": "notVerifier",
      "msg": "Signer is not riff's verifier"
    },
    {
      "code": 6003,
      "name": "invalidConfig",
      "msg": "Invalid config value"
    },
    {
      "code": 6004,
      "name": "invalidArtistId",
      "msg": "Artist ID must be spotify: plus a 22-character Spotify ID"
    },
    {
      "code": 6005,
      "name": "proofMismatch",
      "msg": "A proof doesn't belong to this artist and wallet"
    },
    {
      "code": 6006,
      "name": "proofExpired",
      "msg": "A proof is too old; prove it again"
    },
    {
      "code": 6007,
      "name": "notEnoughProofs",
      "msg": "Needs at least 2 proofs from different sources, including a strong one"
    },
    {
      "code": 6008,
      "name": "tooManyProofs",
      "msg": "Too many proofs"
    },
    {
      "code": 6009,
      "name": "revoked",
      "msg": "The passport has been revoked"
    },
    {
      "code": 6010,
      "name": "notPassportWallet",
      "msg": "Signer is not the passport's wallet"
    },
    {
      "code": 6011,
      "name": "passkeyMissing",
      "msg": "No matching passkey signature in this transaction"
    },
    {
      "code": 6012,
      "name": "passkeyWrongChallenge",
      "msg": "The passkey signature is for a different action"
    },
    {
      "code": 6013,
      "name": "passkeyWrongSite",
      "msg": "The passkey signature is for a different site"
    },
    {
      "code": 6014,
      "name": "passkeyNotVerified",
      "msg": "The passkey wasn't used with the user present and verified"
    },
    {
      "code": 6015,
      "name": "passkeyMalformed",
      "msg": "Malformed passkey data"
    },
    {
      "code": 6016,
      "name": "recoveryPending",
      "msg": "A recovery is already pending"
    },
    {
      "code": 6017,
      "name": "noRecovery",
      "msg": "No recovery is pending"
    },
    {
      "code": 6018,
      "name": "recoveryLocked",
      "msg": "The recovery time-lock hasn't passed yet"
    },
    {
      "code": 6019,
      "name": "notAGuardian",
      "msg": "This proof can't veto the recovery"
    },
    {
      "code": 6020,
      "name": "passkeyRequired",
      "msg": "Withdrawal needs the passkey"
    },
    {
      "code": 6021,
      "name": "insufficientVault",
      "msg": "The vault doesn't hold that much"
    },
    {
      "code": 6022,
      "name": "zeroAmount",
      "msg": "Amount must be greater than zero"
    },
    {
      "code": 6023,
      "name": "mathOverflow",
      "msg": "Math overflow"
    }
  ],
  "types": [
    {
      "name": "coinClaimed",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "passport",
            "type": "pubkey"
          },
          {
            "name": "mint",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "endorsement",
      "docs": [
        "The artist's word on one coin, on any launchpad."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "passport",
            "type": "pubkey"
          },
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "status",
            "type": {
              "defined": {
                "name": "endorsementStatus"
              }
            }
          },
          {
            "name": "updatedAt",
            "type": "i64"
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "endorsementChanged",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "passport",
            "type": "pubkey"
          },
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "status",
            "type": {
              "defined": {
                "name": "endorsementStatus"
              }
            }
          }
        ]
      }
    },
    {
      "name": "endorsementStatus",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "endorsed"
          },
          {
            "name": "disavowed"
          }
        ]
      }
    },
    {
      "name": "passkeyChanged",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "passport",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "passkeyProof",
      "docs": [
        "The parts of a WebAuthn assertion the program needs; the signature and",
        "public key travel in the precompile instruction."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "authenticatorData",
            "type": "bytes"
          },
          {
            "name": "clientDataJson",
            "type": "bytes"
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
      "name": "passportConfig",
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
              "riff's verification service. It checks proofs off-chain (a Spotify",
              "for Artists email, the Spotify bio code, YouTube, a website), attests",
              "them here, and co-signs riff claims."
            ],
            "type": "pubkey"
          },
          {
            "name": "rpIdHash",
            "docs": [
              "sha256 of the passkey site (the WebAuthn relying party, e.g. \"riffpad.fun\")."
            ],
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "recoveryDelay",
            "docs": [
              "How long a recovery waits before it can be finalized, in seconds."
            ],
            "type": "i64"
          },
          {
            "name": "freeWithdrawPerDay",
            "docs": [
              "How much may leave a vault per day without the passkey, in lamports."
            ],
            "type": "u64"
          },
          {
            "name": "proofMaxAge",
            "docs": [
              "How recent proofs must be to issue or recover a passport, in seconds."
            ],
            "type": "i64"
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "passportConfigParams",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "verifier",
            "type": "pubkey"
          },
          {
            "name": "rpIdHash",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "recoveryDelay",
            "type": "i64"
          },
          {
            "name": "freeWithdrawPerDay",
            "type": "u64"
          },
          {
            "name": "proofMaxAge",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "passportIssued",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "passport",
            "type": "pubkey"
          },
          {
            "name": "artistId",
            "type": "string"
          },
          {
            "name": "wallet",
            "type": "pubkey"
          },
          {
            "name": "proofs",
            "type": {
              "vec": {
                "defined": {
                  "name": "proofKind"
                }
              }
            }
          }
        ]
      }
    },
    {
      "name": "passportRevoked",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "passport",
            "type": "pubkey"
          },
          {
            "name": "reason",
            "type": "string"
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
      "name": "proofRecord",
      "docs": [
        "One verified proof that `wallet` speaks for `artist_id`. Proofs are kept",
        "per wallet, so nobody can block an artist by proving first."
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
            "name": "kind",
            "type": {
              "defined": {
                "name": "proofKind"
              }
            }
          },
          {
            "name": "sourceHash",
            "docs": [
              "Hash of what was checked (the email's signature, the bio code, a channel ID, a domain)."
            ],
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
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "proofRecorded",
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
            "name": "kind",
            "type": {
              "defined": {
                "name": "proofKind"
              }
            }
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
      "name": "proofsAdded",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "passport",
            "type": "pubkey"
          },
          {
            "name": "proofs",
            "type": {
              "vec": {
                "defined": {
                  "name": "proofKind"
                }
              }
            }
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
    },
    {
      "name": "recoveryFinalized",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "passport",
            "type": "pubkey"
          },
          {
            "name": "newWallet",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "recoveryRequested",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "passport",
            "type": "pubkey"
          },
          {
            "name": "newWallet",
            "type": "pubkey"
          },
          {
            "name": "effectiveAt",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "recoveryVetoed",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "passport",
            "type": "pubkey"
          },
          {
            "name": "byPasskey",
            "type": "bool"
          },
          {
            "name": "byProof",
            "type": {
              "option": {
                "defined": {
                  "name": "proofKind"
                }
              }
            }
          }
        ]
      }
    },
    {
      "name": "vault",
      "docs": [
        "Holds the artist's fees from every coin they claimed through the passport.",
        "It is the \"artist\" on those coins, and only this program can pay out of it."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "passport",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "vaultWithdrawn",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "passport",
            "type": "pubkey"
          },
          {
            "name": "to",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "withPasskey",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "walletChanged",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "passport",
            "type": "pubkey"
          },
          {
            "name": "oldWallet",
            "type": "pubkey"
          },
          {
            "name": "newWallet",
            "type": "pubkey"
          }
        ]
      }
    }
  ],
  "constants": [
    {
      "name": "configSeed",
      "type": "bytes",
      "value": "[99, 111, 110, 102, 105, 103]"
    },
    {
      "name": "endorsementSeed",
      "type": "bytes",
      "value": "[101, 110, 100, 111, 114, 115, 101]"
    },
    {
      "name": "passportSeed",
      "type": "bytes",
      "value": "[112, 97, 115, 115, 112, 111, 114, 116]"
    },
    {
      "name": "proofSeed",
      "type": "bytes",
      "value": "[112, 114, 111, 111, 102]"
    },
    {
      "name": "vaultSeed",
      "type": "bytes",
      "value": "[118, 97, 117, 108, 116]"
    }
  ]
};
