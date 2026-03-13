package targets

import (
	"testing"

	gateflowv1 "healthd/v1"
)

func TestParseTargets(t *testing.T) {
	tests := []struct {
		name string
		in   []*gateflowv1.NodeList
		want []Target
	}{
		{
			name: "empty input",
			in:   []*gateflowv1.NodeList{},
			want: []Target{},
		},
		{
			name: "maps fields and keeps duplicates",
			in: []*gateflowv1.NodeList{
				{AppId: "a1", AppName: "svc-a", HealthUrl: "http://a/health", ExpectStatus: "200"},
				{AppId: "a1", AppName: "svc-a", HealthUrl: "http://a/health", ExpectStatus: "200"},
			},
			want: []Target{
				{AppUUID: "a1", Name: "svc-a", HealthURL: "http://a/health", ExpectStatus: "200"},
				{AppUUID: "a1", Name: "svc-a", HealthURL: "http://a/health", ExpectStatus: "200"},
			},
		},
		{
			name: "keeps missing and invalid fields as-is",
			in: []*gateflowv1.NodeList{
				{AppId: "", AppName: "", HealthUrl: "not-a-url", ExpectStatus: "abc"},
			},
			want: []Target{
				{AppUUID: "", Name: "", HealthURL: "not-a-url", ExpectStatus: "abc"},
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := ParseTargets(tt.in)
			if len(got) != len(tt.want) {
				t.Fatalf("len(ParseTargets()) = %d, want %d", len(got), len(tt.want))
			}
			for i := range got {
				if got[i] != tt.want[i] {
					t.Fatalf("ParseTargets()[%d] = %+v, want %+v", i, got[i], tt.want[i])
				}
			}
		})
	}
}
