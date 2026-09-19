package transport

import (
	"net/http"
	"net/http/httptest"
	"testing"
	"time"
)

func TestBucketAllowsABurstThenTheRate(t *testing.T) {
	var b bucket
	now := time.Date(2026, 1, 1, 0, 0, 0, 0, time.UTC)
	for i := 0; i < 30; i++ {
		if !b.allow(now, 15, 30) {
			t.Fatalf("command %d of the burst refused", i)
		}
	}
	if b.allow(now, 15, 30) {
		t.Fatal("the 31st command in the same instant should be refused")
	}
	now = now.Add(200 * time.Millisecond) // 3 tokens
	for i := 0; i < 3; i++ {
		if !b.allow(now, 15, 30) {
			t.Fatalf("refill %d refused", i)
		}
	}
	if b.allow(now, 15, 30) {
		t.Fatal("only what was refilled should be allowed")
	}
	now = now.Add(time.Hour)
	if !b.allow(now, 15, 30) || b.tokens > 30 {
		t.Fatalf("tokens after a long pause = %v, want at most the burst", b.tokens)
	}
}

func TestAuthAttemptsAreLimitedPerAddress(t *testing.T) {
	s := &Server{}
	s.Router() // creates the limiter
	clock := time.Date(2026, 1, 1, 0, 0, 0, 0, time.UTC)
	s.authLimiter.now = func() time.Time { return clock }
	handler := s.limitAuth(func(w http.ResponseWriter, r *http.Request) { w.WriteHeader(http.StatusOK) })

	call := func(remote, cf string) int {
		req := httptest.NewRequest("POST", "/auth/login", nil)
		req.RemoteAddr = remote
		if cf != "" {
			req.Header.Set("CF-Connecting-IP", cf)
		}
		rec := httptest.NewRecorder()
		handler(rec, req)
		return rec.Code
	}
	for i := 0; i < int(authBurst); i++ {
		if code := call("1.2.3.4:5000", ""); code != http.StatusOK {
			t.Fatalf("attempt %d: %d", i, code)
		}
	}
	if code := call("1.2.3.4:5001", ""); code != http.StatusTooManyRequests {
		t.Fatalf("over the limit: %d, want 429", code)
	}
	if code := call("5.6.7.8:5000", ""); code != http.StatusOK {
		t.Fatalf("another address must not be affected: %d", code)
	}
	// Without trust in the proxy the header is ignored, so it cannot be used to dodge the limit.
	if code := call("1.2.3.4:5002", "9.9.9.9"); code != http.StatusTooManyRequests {
		t.Fatalf("a forged header dodged the limit: %d", code)
	}
	s.TrustProxyHeaders = true
	if code := call("127.0.0.1:1", "9.9.9.9"); code != http.StatusOK {
		t.Fatalf("behind the proxy the real address counts: %d", code)
	}
	clock = clock.Add(10 * time.Second)
	s.TrustProxyHeaders = false
	if code := call("1.2.3.4:5003", ""); code != http.StatusOK {
		t.Fatalf("the limit should ease with time: %d", code)
	}
}
