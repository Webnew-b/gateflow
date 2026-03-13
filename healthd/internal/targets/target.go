package targets

import (
	"healthd/internal/tools"

	gateflowv1 "healthd/v1"
)

type Target struct {
	AppUUID      string
	Name         string
	HealthURL    string
	ExpectStatus string
}

func ParseTargets(data []*gateflowv1.NodeList) []Target {
	toTarget := func(nl *gateflowv1.NodeList) Target {
		return Target{
			AppUUID:      nl.AppId,
			Name:         nl.AppName,
			HealthURL:    nl.HealthUrl,
			ExpectStatus: nl.ExpectStatus,
		}
	}
	out := tools.MapByPtr(data, toTarget)
	return out
}
