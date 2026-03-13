package targets

import "math/bits"

type Slot[T any] struct {
	data [8]T
	mask uint8
}

func (s *Slot[T]) IsEmpty() bool { return s.mask == 0 }

func (s *Slot[T]) IsFull() bool { return s.mask == 0xFF }

func (s *Slot[T]) Len() int { return bits.OnesCount8(s.mask) }

func (s Slot[T]) Unpack() []T {
	out := make([]T, 0, bits.OnesCount8(s.mask))
	for i := 0; i < len(s.data); i++ {
		if (s.mask & (1 << uint(i))) == 0 {
			continue
		}
		out = append(out, s.data[i])
	}
	return out
}

func (s *Slot[T]) GetLastRef() *T {
	return &s.data[len(s.data)-1]
}

func MapSlot[T any, Z any](s Slot[T], f func(t T) Z) Slot[Z] {
	var z Slot[Z]
	for _, i := range s.Unpack() {
		zi := f(i)
		_, _ = z.TryAdd(zi)
	}
	return z
}

func (s *Slot[T]) TryAdd(v T) (idx int, ok bool) {
	for i := 0; i < 8; i++ {
		if (s.mask & (1 << uint(i))) == 0 {
			s.data[i] = v
			s.mask |= (1 << uint(i))
			return i, true
		}
	}
	return -1, false
}

func NewTargetSlot(t []Target) []Slot[Target] {
	if len(t) == 0 {
		return []Slot[Target]{}
	}
	var ts []Slot[Target]
	var tsi Slot[Target]

	for _, i := range t {
		if _, ok := tsi.TryAdd(i); !ok {
			ts = append(ts, tsi)
			tsi = Slot[Target]{}
			_, _ = tsi.TryAdd(i)
		}
	}
	if !tsi.IsEmpty() {
		ts = append(ts, tsi)
	}
	return ts
}
