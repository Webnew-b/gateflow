package tools

func Map[A any, B any](lista []A, f func(a A) B) []B {
	var listb []B
	for _, i := range lista {
		listb = append(listb, f(i))
	}
	return listb
}

func MapByPtr[A any, B any](lista []*A, f func(a *A) B) []B {
	var listb []B
	for _, i := range lista {
		listb = append(listb, f(i))
	}
	return listb
}
