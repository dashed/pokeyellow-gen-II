; Boolean checks
FALSE EQU 0
TRUE  EQU 1

; flag operations
	const_def
	const FLAG_RESET ; 0
	const FLAG_SET   ; 1
	const FLAG_TEST  ; 2

; wOptions
TEXT_DELAY_MASK       EQU %1111
TEXT_DELAY_WARP       EQU %000 ; 0
TEXT_DELAY_FAST       EQU %001 ; 1
TEXT_DELAY_MEDIUM     EQU %011 ; 3
TEXT_DELAY_SLOW       EQU %101 ; 5
