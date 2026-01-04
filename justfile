_:
  @just -l

mon port:
  rlwrap --always-readline --no-child --ansi-colour-aware picocom -cqb 115200 {{port}}
