FUNCTION_BLOCK OPENOT_TestConformantMessage
VAR_OUTPUT
    Passed : BOOL;
    MismatchIndex : UINT;
    ActualLength : UINT;
    ExpectedLength : UINT;
END_VAR
VAR
    Encoder : OPENOT_EncodeConformantMessage;
    Expected : ARRAY[0..75] OF BYTE := [
        BYTE#16#4F, BYTE#16#4F, BYTE#16#54, BYTE#16#32, BYTE#16#4C, BYTE#16#00, BYTE#16#08, BYTE#16#00,
        BYTE#16#0A, BYTE#16#CA, BYTE#16#9A, BYTE#16#3B, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#01, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#02, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#42, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#03, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#14, BYTE#16#00, BYTE#16#05, BYTE#16#04, BYTE#16#E9, BYTE#16#03, BYTE#16#00, BYTE#16#00,
        BYTE#16#15, BYTE#16#00, BYTE#16#0C, BYTE#16#0B, BYTE#16#70, BYTE#16#68, BYTE#16#61, BYTE#16#73,
        BYTE#16#65, BYTE#16#20, BYTE#16#72, BYTE#16#65, BYTE#16#61, BYTE#16#64, BYTE#16#79, BYTE#16#00,
        BYTE#16#08, BYTE#16#00, BYTE#16#03, BYTE#16#02, BYTE#16#F4, BYTE#16#01, BYTE#16#00, BYTE#16#00,
        BYTE#16#CF, BYTE#16#CF, BYTE#16#BE, BYTE#16#B0
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
