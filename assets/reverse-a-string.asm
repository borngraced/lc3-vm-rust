;; reverse a string
        .ORIG    X3000
REV     LEA      R0,FILE      ;; R0 IS BEGINNING OF STRING
        ADD      R1,R0,#-1    
LOOP1   LDR      R3,R1,#1     ;; NOTE -- LDR "LOOKS" AT THE WORD PAST R1
        BRZ      DONE1
        ADD      R1,R1,#1
        BR       LOOP1

DONE1   NOT      R2,R0
        ADD      R2,R2,R1

;; r0 == address of first character of string
;; r1 == address of last character of string
;; r2 == size of string - 2  (think about it....)
LOOP2   ADD      R2,R2,#0
        BRN      DONE2
        LDR      R3,R0,#0     ;; SWAP
        LDR      R4,R1,#0
        STR      R4,R0,#0
        STR      R3,R1,#0
        ADD      R0,R0,#1     ;; MOVE POINTERS
        ADD      R1,R1,#-1
        ADD      R2,R2,#-2    ;; DECREASE R2 BY 2
        BR       LOOP2

DONE2   PUTS
        HALT


FILE    .stringz "this is so much fun!"
        .end

