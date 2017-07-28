package wav

import (
	"fmt"
	"testing"
)

func TestLoad(t *testing.T) {
	wf, err := Load("../downloader/out.wav")
	if err != nil {
		t.Error(err)
	}

	fmt.Println(wf)
}
