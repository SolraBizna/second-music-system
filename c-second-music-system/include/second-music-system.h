#ifndef SECOND_MUSIC_SYSTEM_H
#define SECOND_MUSIC_SYSTEM_H

#include <stdlib.h>

#if __cplusplus
extern "C" {
#endif

struct SMS_Soundtrack;

// Error handling:
// `error_out` parameter, if non-NULL, is filled in with a newly malloc'd C
// string containing the error text (including a null terminator). You must
// free this when you're done with it.
// `error_len_out` parameter, if non-NULL, is filled in with the length of the
// new error (not including the null terminator).
// If no error occurs, they are *not touched*. Functions that return pointers
// will return NULL on error. Functions that return int will return 0 on
// error.

SMS_Soundtrack* SMS_new_soundtrack();
void SMS_free_soundtrack(SMS_Soundtrack*);

// If parsing fails, returns NULL.
SMS_Soundtrack* SMS_parse_new_soundtrack(const char* src, size_t src_len, char** error_out, size_t* error_len_out);
SMS_Soundtrack* SMS_parse_new_soundtrack_str(const char* src, char** error_out, size_t* error_len_out);
// Changes the soundtrack, adding new elements and replacing same-named ones.
// If parsing fails, leaves the existing soundtrack alone.
int SMS_parse_soundtrack(SMS_Soundtrack*, const char* src, size_t src_len, char** error_out, size_t* error_len_out);
int SMS_parse_soundtrack_str(SMS_Soundtrack*, const char* src, char** error_out, size_t* error_len_out);

#if __cplusplus
}
#endif

#endif
