FUNCTION_BLOCK OPENOT_TestDefinitionChanged
VAR_OUTPUT
    Passed : BOOL;
    MismatchIndex : UINT;
    ActualLength : UINT;
    ExpectedLength : UINT;
END_VAR
VAR
    Encoder : OPENOT_EncodeDefinitionChanged;
    DefHashOld : OPENOT_HASH8 := [
        BYTE#16#6F, BYTE#16#6C, BYTE#16#64, BYTE#16#68,
        BYTE#16#61, BYTE#16#73, BYTE#16#68, BYTE#16#31
    ];
    DefHashNew : OPENOT_HASH8 := [
        BYTE#16#6E, BYTE#16#65, BYTE#16#77, BYTE#16#68,
        BYTE#16#61, BYTE#16#73, BYTE#16#68, BYTE#16#32
    ];
    Expected : ARRAY[0..79] OF BYTE := [
        BYTE#16#4F, BYTE#16#4F, BYTE#16#54, BYTE#16#32, BYTE#16#50, BYTE#16#00, BYTE#16#08, BYTE#16#00,
        BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#01, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#01, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#06, BYTE#16#01, BYTE#16#00, BYTE#16#00,
        BYTE#16#39, BYTE#16#00, BYTE#16#0D, BYTE#16#08, BYTE#16#6F, BYTE#16#6C, BYTE#16#64, BYTE#16#68,
        BYTE#16#61, BYTE#16#73, BYTE#16#68, BYTE#16#31, BYTE#16#1C, BYTE#16#00, BYTE#16#0D, BYTE#16#08,
        BYTE#16#6E, BYTE#16#65, BYTE#16#77, BYTE#16#68, BYTE#16#61, BYTE#16#73, BYTE#16#68, BYTE#16#32,
        BYTE#16#3A, BYTE#16#00, BYTE#16#07, BYTE#16#08, BYTE#16#02, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#A5, BYTE#16#41, BYTE#16#2E, BYTE#16#B1
    ];
    Index : UINT;
END_VAR

Encoder(
    RunId := ULINT#1,
    Seq := ULINT#1,
    DefHashOld := DefHashOld,
    DefHashNew := DefHashNew,
    EpochId := ULINT#2,
    SourceTime := ULINT#0);
ActualLength := Encoder.Length;
ExpectedLength := UINT#80;
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
