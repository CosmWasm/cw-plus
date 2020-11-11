package version

import (
	"fmt"
	"runtime"
)

// AppName represents the application name as the 'user agent' on the larger Ethereum network.
const AppName = "Aragon Chain"

// Version contains the application semantic version.
const Version = "0.1.0"

// ProtocolVersion is the supported Ethereum protocol version (e.g., Homestead, Olympic, etc.)
const ProtocolVersion = 63

// GitCommit contains the git SHA1 short hash set by build flags.
var GitCommit = ""

// ClientVersion returns the full version string for identification on the larger Ethereum network.
func ClientVersion() string {
	return fmt.Sprintf("%s/%s+%s/%s/%s", AppName, Version, GitCommit, runtime.GOOS, runtime.Version())
}
