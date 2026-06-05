FUNCTION_BLOCK OPENOT_TestConformantStateTransition
VAR_OUTPUT
    Passed : BOOL;
    MismatchIndex : UINT;
    ActualLength : UINT;
    ExpectedLength : UINT;
END_VAR
VAR
    Encoder : OPENOT_EncodeConformantStateTransition;
    Expected : ARRAY[0..75] OF BYTE := [
        BYTE#16#4F, BYTE#16#4F, BYTE#16#54, BYTE#16#32, BYTE#16#4C, BYTE#16#00, BYTE#16#08, BYTE#16#00,
        BYTE#16#00, BYTE#16#CA, BYTE#16#9A, BYTE#16#3B, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#01, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#01, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#42, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#01, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#01, BYTE#16#00, BYTE#16#05, BYTE#16#04, BYTE#16#07, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#02, BYTE#16#00, BYTE#16#03, BYTE#16#02, BYTE#16#02, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#03, BYTE#16#00, BYTE#16#03, BYTE#16#02, BYTE#16#03, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#04, BYTE#16#00, BYTE#16#03, BYTE#16#02, BYTE#16#04, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#60, BYTE#16#A3, BYTE#16#A5, BYTE#16#47
    ];
    Index : UINT;
END_VAR

Encoder();
ActualLength := Encoder.Length;
ExpectedLength := UINT#76;
Passed := ActualLength = ExpectedLength;
MismatchIndex := UINT#65535;

IF Passed THEN
    FOR Index := UINT#0 TO ExpectedLength - UINT#1 DO
        IF (Encoder.Buffer[Index] <> Expected[Index]) AND (MismatchIndex = UINT#65535) THEN
            Passed := FALSE;
            MismatchIndex := Index;
        END_IF;
    END_FOR;
END_IF;
END_FUNCTION_BLOCK
