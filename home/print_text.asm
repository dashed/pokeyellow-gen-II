; This function is used to wait a short period after printing a letter to the
; screen unless the player presses the A/B button or the delay is turned off
; through the [wd730] or [wLetterPrintingDelayFlags] flags.
PrintLetterDelay::
	ld a, [wd730]
	bit 6, a
	ret nz

	; non-scrolling text?	
	ld a, [wLetterPrintingDelayFlags]
	bit 1, a
	ret z
	
	push hl
	push de
	push bc

	; force fast scroll?
	ld a, [wLetterPrintingDelayFlags]
	bit 0, a
	jr z, .waitOneFrame

	; text speed
	ld a, [wOptions]
	and TEXT_DELAY_MASK ; 1111 mask
	jr z, .done ; exit early if the text speed is warp mode.
	ldh [hFrameCounter], a
	jr .checkButtons
.waitOneFrame
	ld a, 1
	ldh [hFrameCounter], a
.checkButtons
	call Joypad
	ldh a, [hJoyHeld]
.checkAButton
	bit 0, a ; is the A button pressed?
	jr z, .checkBButton
	jr .endWait
.checkBButton
	bit 1, a ; is the B button pressed?
	jr z, .buttonsNotPressed
.endWait
	call DelayFrame
	jr .done
.buttonsNotPressed ; if neither A nor B is pressed
	ldh a, [hFrameCounter]
	and a
	jr nz, .checkButtons
.done
	pop bc
	pop de
	pop hl
	ret
