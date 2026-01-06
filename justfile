_:
  @just -l

mon port:
  rlwrap --always-readline --no-child --ansi-colour-aware picocom -cqb 115200 {{port}}
  # rlwrap --always-readline --ansi-colour-aware arduino-cli monitor --port {{port}} -c 115200
