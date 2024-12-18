#pragma once

void *memcpy(void *dest, const void *src, unsigned long n);
void *memmove(void *dest, const void *src, unsigned long n);
void *memset(void *s, int c, unsigned long n);
int memcmp(const void *ptr1, const void *ptr2, unsigned long n);
int strncmp(const char *s1, const char *s2, unsigned long n);
