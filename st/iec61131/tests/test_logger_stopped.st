FUNCTION_BLOCK OPENOT_TestLoggerStopped
VAR_OUTPUT
    Passed : BOOL;
    MismatchIndex : UINT;
    ActualLength : UINT;
    ExpectedLength : UINT;
END_VAR
VAR
    Encoder : OPENOT_EncodeLoggerStopped;
    Expected : ARRAY[0..43] OF BYTE := [
        BYTE#16#4F, BYTE#16#4F, BYTE#16#54, BYTE#16#32, BYTE#16#2C, BYTE#16#00, BYTE#16#08, BYTE#16#00,
        BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#01, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#02, BYTE#16#01, BYTE#16#00, BYTE#16#00,
        BYTE#16#09, BYTE#16#A8, BYTE#16#70, BYTE#16#85
    ];
    Index : UINT;
END_VAR

Encoder(RunId := ULINT#1, Seq := ULINT#0, SourceTime := ULINT#0);
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
