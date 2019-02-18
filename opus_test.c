#include <opus.h>
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    int err;
    OpusEncoder* enc = opus_encoder_create(48000, 2, OPUS_APPLICATION_AUDIO, &err);

    if (err != NULL) {
        puts("initializing opus");
        return EXIT_FAILURE;
    }



    return EXIT_SUCCESS;
}