#ifndef VAM_I18N_H
#define VAM_I18N_H
#include <stddef.h>
#define LANG_COUNT 4
const char *tr(const char *key);
const char *language_name(void);
void language_next(void);
const char *tr_message(const char *message);
const char *tr_subtitle(int kind, const char *value, char *buffer, size_t size, int detail);
#endif
