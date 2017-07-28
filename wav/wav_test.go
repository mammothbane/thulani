package wav

import (
	"fmt"
	"testing"
)

func TestLoad(t *testing.T) {
	ch, err := Load("../downloader/out.wav")
	if err != nil {
		t.Fatal(err)
	}

	ct := 0

	for _ = range ch {
		//fmt.Println(i)
		ct++
		if ct%10000 == 0 {
			fmt.Println(ct)
		}
	}

	fmt.Println("COUNT: ", ct)
}
