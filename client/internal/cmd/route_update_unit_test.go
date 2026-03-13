package cmd

import "testing"

func TestRouteUpdateVerify_AcceptsValidRequest(t *testing.T) {
	err := RouteUpdateVerify(RouteUpdateReq{
		AppName:      "demo-app",
		MountPath:    "/demo",
		UpstreamPath: "/api",
	})
	if err != nil {
		t.Fatalf("RouteUpdateVerify() error = %v, want nil", err)
	}
}

func TestRouteUpdateVerify_RejectsInvalidRequest(t *testing.T) {
	cases := []RouteUpdateReq{
		{AppName: "", MountPath: "/demo", UpstreamPath: "/api"},
		{AppName: "demo", MountPath: "demo", UpstreamPath: "/api"},
		{AppName: "demo", MountPath: "/demo", UpstreamPath: "api"},
	}

	for i, c := range cases {
		if err := RouteUpdateVerify(c); err == nil {
			t.Fatalf("case %d: expected error, got nil", i)
		}
	}
}
