#pragma once

// just some filler type
#define FILE void

#define stdin NULL
#define stdout NULL
#define stderr NULL

int fprintf(FILE *__restrict__, const char *__restrict__, ...);
int fputs(const char *__restrict, FILE *__restrict);
int fputc(int, FILE *);
FILE *fdopen(int, const char *);
int fclose(FILE *);

int vsnprintf(char *s, unsigned long n, const char *format, ...);

#define sprintf(str, ...) 0
#define snprintf(str, len, ...) 0
