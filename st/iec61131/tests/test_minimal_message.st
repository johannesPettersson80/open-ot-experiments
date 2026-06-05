FUNCTION_BLOCK OPENOT_TestMinimalMessage
VAR_OUTPUT
    Passed : BOOL;
    MismatchIndex : UINT;
    ActualLength : UINT;
    ExpectedLength : UINT;
END_VAR
VAR
    Encoder : OPENOT_EncodeMinimalMessage;
    Expected : ARRAY[0..43] OF BYTE := [
        BYTE#16#4F, BYTE#16#4F, BYTE#16#54, BYTE#16#32, BYTE#16#2C, BYTE#16#00, BYTE#16#08, BYTE#16#00,
        BYTE#16#03, BYTE#16#00, BYTE#16#32, BYTE#16#B4, BYTE#16#91, BYTE#16#D4, BYTE#16#B3, BYTE#16#18,
        BYTE#16#01, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#03, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#07, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#03, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#21, BYTE#16#52, BYTE#16#EF, BYTE#16#97
    ];
    Index : UINT;
END_VAR

Encoder(
    SourceId := UDINT#7,
    RunId := ULINT#1,
    Seq := ULINT#3,
    SourceTime := ULINT#1780000000000000003);
ActualLength := Encoder.Length;
ExpectedLength := UINT#44;
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
