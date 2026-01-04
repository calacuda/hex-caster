_:
  @just -l

mon port:
  rlwrap --always-readline picocom -b 115200 {{port}}
