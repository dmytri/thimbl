Feature: Harbor methodology conformance
  As the Shipshape workflow itself
  I want executable checks of my own methodology rules
  So that violations become failing verification targets instead of silent drift

  @conformance
  Scenario: The watchbill respects its fixed shape
    Given the watchbill file at "watchbill.json" when present
    When the verifier reads every watch object
    Then every key matches "watch<number>" with only a "scenarios" array
    And every reference follows the "<spec>.feature:<Scenario Name>" form
    And an absent watchbill conforms as the deck at rest

  @conformance
  Scenario: The implementation carries no standing perturbation token
    Given the implementation directory "src"
    When the verifier searches every source file for the token "PERTURBATION"
    Then no match is found

  @conformance
  Scenario: Every implementation plank names a current step pattern
    Given the implementation directory "src"
    And the step definitions at "tests/cucumber/definitions.rs"
    When the verifier joins every plank against the step patterns
    Then every plank string matches a step pattern
    And every plank token sits in a declaration docblock
