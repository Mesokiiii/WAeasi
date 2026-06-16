package wasi

import "time"

// MonotonicNow returns nanoseconds since instance start.
//
// The pipeline replaces this with a direct
// wasi:clocks/monotonic-clock.now() call at build time.  In dev mode
// (running under standard Go) we use time.Since against the package
// init timestamp.
var MonotonicNow = monotonicFallback

// WallNow returns Unix-epoch nanoseconds.
var WallNow = wallFallback

var monoBase = time.Now()

func monotonicFallback() int64 {
	return time.Since(monoBase).Nanoseconds()
}

func wallFallback() int64 {
	return time.Now().UnixNano()
}
