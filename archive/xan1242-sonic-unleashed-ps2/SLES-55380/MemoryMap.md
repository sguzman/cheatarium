# Memory mapping

This table describes the memory mapping outside the code segment/area.

This is focused on reclaimed/taken space from unused areas.

NOTE: this may not be accurate to the latest version/commit as it's updated by hand.

| **Start Address** | **End Address** | **Size** | Type | **Description**                                                |
| ----------------- | --------------- | -------- | ---- | -------------------------------------------------------------- |
| 76D4F0            | 76D50C          | 1C       | Hook | Disable Chip's Messages                                        |
| 76D50C            | 76D524          | 18       | Hook | Disable Werehog Battle Music                                   |
| 76D524            | 76D534          | 10       | Hook | global font spacing                                            |
| 76D534            | 76D554          | 20       | Hook | lightspeed dash boost ender hook                               |
| 76D554            | 76D570          | 1C       | Hook | PlayBGM wrapper                                                |
| 7F7DF0            | 7F7E58          | 68       | Var  | sprintf buffer for NoAFS byID                                  |
| 7CB580            | 7CB598          | 18       | Var  | sprintf format string for NoAFS byID                           |
| 7CB598            | 7CB5C0          | 28       | Free | free space                                                     |
| 820478            | 82047C          | 4        | Var  | global font spacing value                                      |
| 82047C            | 820480          | 4        | Var  | ultrawide movie player mov_r                                   |
| 820480            | 820484          | 4        | Var  | ultrawide movie player mov_l                                   |
| 820484            | 820488          | 4        | Var  | NPCTalk X scale for ws/uw                                      |
| 820488            | 82048C          | 4        | Var  | NPCTalk X bg translation for uw                                |
| 82048C            | 820490          | 4        | Var  | Continuous boost drain amount                                  |
| 820490            | 820494          | 4        | Var  | Continuous boost initial drain                                 |
| 820498            | 8204A4          | C        | Free | free space                                                     |
| 8204A4            | 8204A8          | 4        | Var  | Boost cancel var (needs to be here because initial value is 0) |
| 8204A8            | 820500          | 58       | Free | free space                                                     |
| 820500            | 820504          | 4        | Var  | "%s\\n" string for printf helpers                              |
| 820504            | 82051C          | 18       | Hook | fix NPCTalk rendering                                          |
| 82051C            | 82052C          | 10       | Hook | DrawMovieWithPos hook for uw                                   |
| 82052C            | 820548          | 1C       | Hook | Sonic Control Remap - enablement check func                    |
| 820548            | 82056C          | 24       | Hook | Sonic Control Remap - one-frame button down func               |
| 82056C            | 820588          | 1C       | Hook | Sonic Control Remap - current button down func                 |
| 820588            | 8205A8          | 20       | Hook | Werehog Unleash prevent drain during cutscenes                 |
| 8205A8            | 8205B4          | C        | Hook | 60FPS CCameraChanger dT hook                                   |
| 8205B4            | 8205F0          | 3C       | Hook | 60FPS GimSonicSpring per-obj half-rate hack func               |
| 8205F0            | 820600          | 10       | Hook | Common voice clip trampoline                                   |
| 820600            | 820620          | 20       | Hook | Boost voice clip selector                                      |
| 820620            | 820634          | 14       | Hook | Drift voice clip selector                                      |
| 820634            | 820644          | 10       | Hook | Jump voice clip selector                                       |
| 820644            | 820668          | 24       | Hook | Other voice clip selector                                      |
| 820668            | 82068C          | 24       | Hook | Werehog howl on unleash hook                                   |
| 82068C            | 8206B4          | 28       | Hook | JP voice sound bank ID patcher                                 |
| 8206B4            | 8206D0          | 1C       | Free | free space                                                     |
| 836CF0            | 836E10          | 120      | Var  | UnwiishedPS2Debug - nnsPrintSvrObj                             |
| 836E10            | 836E1C          | C        | Hook | bGetTicker                                                     |
| 836E1C            | 836E74          | 58       | Hook | bGetTickerDifference                                           |
| 836E74            | 836EE8          | 74       | Hook | Custom frametime sync                                          |
| 836EE8            | 836EFC          | 14       | Hook | Frametime initialize value hook                                |
| 836EFC            | 837450          | 554      | Free | free space                                                     |
