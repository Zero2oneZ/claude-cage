# POST Beep Codes & Audio Feedback

## What You'll Hear (Success Path)

### BIOS POST (Power On)
| Beeps | Meaning |
|-------|---------|
| 1 short | POST successful, booting |
| None | Also usually OK (some boards are silent on success) |

### Boot Progress (from our scripts)
Once Ubuntu starts, we play audio tones through the PC speaker:

| Sound | Stage |
|-------|-------|
| 2 short beeps | Forensic scan starting |
| 3 short beeps | Forensic scan complete, starting install |
| 4 short beeps | Ubuntu install complete, rebooting |
| 5 short beeps | First boot into Ubuntu, SSH available |
| Long beep | Something went wrong (check logs) |

## Common BIOS Error Beep Codes

### AMI BIOS (most common)
| Beeps | Problem |
|-------|---------|
| 1 short | DRAM refresh failure |
| 2 short | Parity error |
| 3 short | Base 64K RAM failure |
| 4 short | System timer failure |
| 5 short | CPU failure |
| 6 short | Keyboard controller failure |
| 7 short | Virtual mode exception |
| 8 short | Display memory failure |
| 9 short | ROM BIOS checksum failure |
| 10 short | CMOS shutdown register failure |
| 11 short | Cache memory failure |
| 1 long, 2 short | Video card failure |
| 1 long, 3 short | Memory test failure |
| Continuous | Memory or video problem |

### Award BIOS
| Beeps | Problem |
|-------|---------|
| 1 long, 2 short | Video error |
| 1 long, 3 short | Video error |
| Continuous | Memory error |
| Repeating high-low | CPU overheating |

### Phoenix BIOS (beep-pause-beep pattern)
| Pattern | Problem |
|---------|---------|
| 1-1-3 | CMOS read/write failure |
| 1-1-4 | ROM BIOS checksum error |
| 1-2-1 | Programmable interval timer failure |
| 1-2-2 | DMA initialization failure |
| 1-2-3 | DMA page register failure |
| 1-3-1 | RAM refresh failure |
| 3-1-1 | Slave DMA register failure |
| 3-1-2 | Master DMA register failure |
| 3-2-4 | Keyboard controller failure |
| 4-2-3 | Gate A20 failure |

## No Beeps At All?

### Possible causes:
1. **No PC speaker connected** — Many modern cases don't include one. You can buy a cheap motherboard speaker ($2) that plugs into the SPEAKER header.
2. **Speaker header disabled** — Some BIOS have an option to disable it.
3. **Dead board** — No power reaching motherboard.
4. **PSU issue** — Check PSU switch and power cable.

### What to check:
- Power LED on motherboard lit?
- Fans spinning?
- Any LED debug codes on the board? (Some boards have a 2-digit LED display)

## Interpreting Boot Progress Without Monitor

### Timeline (approximate):
```
0:00  Power on
0:02  POST beep (if enabled) — BIOS is alive
0:05  BIOS looking for boot device
0:10  Should start booting from USB/PXE
0:30  Ubuntu installer loading
1:00  Forensic scan starts (2 beeps if we added speaker support)
5:00  Forensic scan complete (3 beeps)
5:30  Disk partitioning + formatting
10:00 Package installation
20:00 Install complete, reboot (4 beeps)
21:00 First boot into Ubuntu (5 beeps)
21:30 SSH available — try `make find`
```

### If nothing happens after 2 minutes:
- No boot device found (BIOS boot order issue)
- Try: power cycle, check USB is fully seated
- Last resort: need monitor to check BIOS

## Adding a PC Speaker

If your case doesn't have one, buy a "motherboard speaker" or "PC speaker buzzer":
- ~$2 on Amazon/eBay
- 4-pin connector, plugs into SPEAKER header on motherboard
- Usually near the front panel connectors

This is the ONLY way to get audio feedback from BIOS before the OS loads.
