FUNCTION_BLOCK OPENOT_TestLoggerStartedWarm
VAR_OUTPUT
    Passed : BOOL;
    MismatchIndex : UINT;
    ActualLength : UINT;
    ExpectedLength : UINT;
END_VAR
VAR
    Encoder : OPENOT_EncodeLoggerStarted;
    Expected : ARRAY[0..51] OF BYTE := [
        BYTE#16#4F, BYTE#16#4F, BYTE#16#54, BYTE#16#32, BYTE#16#34, BYTE#16#00, BYTE#16#08, BYTE#16#00,
        BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#01, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#02, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#01, BYTE#16#01, BYTE#16#00, BYTE#16#00,
        BYTE#16#3B, BYTE#16#00, BYTE#16#00, BYTE#16#01, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#C7, BYTE#16#02, BYTE#16#02, BYTE#16#E3
    ];
    Index : UINT;
END_VAR

Encoder(RunId := ULINT#1, Seq := ULINT#2, ColdStart := FALSE, SourceTime := ULINT#0);
ActualLength := Encoder.Length;
ExpectedLength := UINT#52;
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
