package transport

import (
	"net"
	"net/http"
	"strconv"
	"strings"
	"sync"
	"time"
)

// bucket is a token bucket: it allows `burst` events at once and then `rate` per second.
type bucket struct {
	tokens float64
	last   time.Time
}

func (b *bucket) allow(now time.Time, rate, burst float64) bool {
	if b.last.IsZero() {
		b.tokens = burst
	} else {
		b.tokens = min(burst, b.tokens+now.Sub(b.last).Seconds()*rate)
	}
	b.last = now
	if b.tokens < 1 {
		return false
	}
	b.tokens--
	return true
}

// Commands from one game connection: a human clicking never gets near this, a script flooding
// the world does.
const (
	commandRate  = 15.0 // per second, sustained
	commandBurst = 30.0
	// Consecutive refused commands after which the connection is cut instead of only ignored.
	commandAbuseLimit = 200
)

// Login and registration attempts from one address. Passwords are hashed with bcrypt, so guessing
// is already slow; this keeps a single address from hammering it (and the CPU) anyway.
const (
	authRate  = 1.0 / 3 // per second, sustained: 20 a minute
	authBurst = 20.0
)

// ipLimiter limits requests per client address.
type ipLimiter struct {
	mu      sync.Mutex
	buckets map[string]*bucket
	now     func() time.Time
}

func newIPLimiter() *ipLimiter {
	return &ipLimiter{buckets: map[string]*bucket{}, now: time.Now}
}

func (l *ipLimiter) allow(key string) bool {
	l.mu.Lock()
	defer l.mu.Unlock()
	now := l.now()
	if len(l.buckets) > 5000 { // forget addresses that have been quiet long enough to be full again
		for k, b := range l.buckets {
			if now.Sub(b.last) > time.Duration(authBurst/authRate)*time.Second {
				delete(l.buckets, k)
			}
		}
	}
	b := l.buckets[key]
	if b == nil {
		b = &bucket{}
		l.buckets[key] = b
	}
	return b.allow(now, authRate, authBurst)
}

// clientIP is the address requests are counted against. Behind a trusted proxy (Cloudflare Tunnel)
// every connection comes from the proxy itself, so the real address is in a header; without one
// that header could be forged, so it is used only when TrustProxyHeaders is set.
func (s *Server) clientIP(r *http.Request) string {
	if s.TrustProxyHeaders {
		if ip := strings.TrimSpace(r.Header.Get("CF-Connecting-IP")); ip != "" {
			return ip
		}
	}
	host, _, err := net.SplitHostPort(r.RemoteAddr)
	if err != nil {
		return r.RemoteAddr
	}
	return host
}

// limitAuth refuses login/registration attempts from addresses that make too many.
func (s *Server) limitAuth(next http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if !s.authLimiter.allow(s.clientIP(r)) {
			w.Header().Set("Retry-After", strconv.Itoa(int(1/authRate)))
			writeError(w, http.StatusTooManyRequests, "troppi tentativi: riprova tra qualche secondo")
			return
		}
		next(w, r)
	}
}
