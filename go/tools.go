//go:build tools
package tools

import (
    _ "github.com/moby/spdystream"
    _ "github.com/sirupsen/logrus"
    _ "golang.org/x/crypto/ssh"
    _ "golang.org/x/oauth2"
)
