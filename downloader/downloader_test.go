package downloader

import (
	"fmt"
	"testing"
	"time"
)

func TestGetUrl(t *testing.T) {
	u, err := getUrl("https://www.youtube.com/watch?v=_K13GJkGvDw")
	if err != nil {
		t.Fatal(err)
	}

	fmt.Println(u)
}

func TestDownload(t *testing.T) {
	if _, err := Download("https://www.youtube.com/watch?v=_K13GJkGvDw", 10*time.Second, 10*time.Second); err != nil {
		t.Fatal(err)
	}
}
