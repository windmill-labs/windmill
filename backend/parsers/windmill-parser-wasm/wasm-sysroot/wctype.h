#pragma once

typedef __WCHAR_TYPE__ wchar_t;
typedef __WINT_TYPE__ wint_t;

int iswspace(wchar_t ch);
int iswalnum(wint_t _wc);

// Manually added (needed for Ruby) 
int iswdigit(wint_t _wc);
int iswupper(wint_t _wc);
int iswalpha(wint_t _wc);
int iswlower(wint_t _wc);
