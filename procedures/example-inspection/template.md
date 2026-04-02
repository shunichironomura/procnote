---
id: INSP-001
title: "Visual Inspection - PCB Assembly"
version: "1.0"
author: "Nomura"
---

## Prepare workspace

Ensure the inspection area is clean and well-lit.

- [ ] Wear ESD wrist strap
- [ ] Clean inspection surface

## Inspect solder joints

Use a magnifying glass or microscope to inspect all solder joints.

- [ ] No cold solder joints
- [ ] No solder bridges
- [ ] No missing components

## Record results

```inputs
- id: result
  type: selection
  label: "Inspection result"
  options:
    - "Pass"
    - "Fail"
    - "Conditional pass"
- id: notes
  type: text
  label: "Inspector notes"
```
