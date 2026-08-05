D-10 is fixed. `redde °CCC` exits 44 and always did — Unix keeps an exit
status's low eight bits, and the packages design §4.3 specifies the truncation,
so the behaviour is deliberate and stays. What was missing was any sign that it
happened: `redde °CCC` exited 44 in silence, and `redde °CCLVI` exited 0, which
reads as success.

The returned value is now compared against the status actually delivered, and a
`Monitum` is emitted when they differ. `redde °III` says nothing.

The truncated value is rendered through `Roman::display`, so no Arabic digit
reaches the message.
