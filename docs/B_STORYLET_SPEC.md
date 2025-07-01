# Sutra Storylets: Formal Specification

---

## **I. Purpose and Role**

- **Storylets** are modular, eligibility-driven narrative or game units.
    
- They represent _events, encounters, scenes, or micro-narratives_ that are triggered based on world/agent state, pool membership, and eligibility logic.
    
- **Storylets are the foundational "atom" of emergent, replayable, simulation-driven content in Sutra.**
    

---

## **II. Structure and Declaration**

### **Canonical Syntax**

```sutra
storylet "id" {
  [display "Optional human-readable name"]
  [tags ...]          ; e.g., tag social, tag danger, etc.
  [history]           ; if this storylet should be tracked as "seen"
  [weight ...]        ; eligibility or selection weighting (prefix expression)
  [when ...]          ; eligibility/gating condition (prefix boolean expr)
  [other metadata]
  [body ...]          ; core narrative, choices, effects (see below)
}
```

**Examples:**

```sutra
storylet "duel" {
  tag social
  weight (* agent.rivalry 2)
  when (> agent.rivalry 2)
  history
  print "A bitter rivalry erupts into a duel!"
  set! agent.wounds (+ agent.wounds 1)
}
```

---

## **III. Eligibility and Selection**

- **Eligibility**:
    
    - Evaluated via the `when` field (if present), using a prefix boolean expression.
        
    - Defaults to “eligible” if no `when` field specified.
        
    - Context: eligibility is always evaluated in the scope of the relevant agent, player, or world (according to pool construction).
        
- **Weighting**:
    
    - Used when selecting among multiple eligible storylets (via `select`, `random`, `sample` macros, etc.).
        
    - Evaluated as a prefix expression; default is `1` if unspecified.
        
- **Tags/Metadata**:
    
    - Used for pool construction, filtering, and analytic purposes (e.g., `tag social`, `region north`).
        

---

## **IV. History and Seen Tracking**

- **History**:
    
    - If the `history` keyword is present, firing/executing this storylet records it as “seen” for the current agent/player/session (as appropriate).
        
    - The system can use this to filter future pools (`exclude seen`), trigger recaps, or branch based on prior experience.
        
    - **History checks** (e.g., `if (seen "duel") ...`) are prefix atoms/macros and can be used in any logic or text.
        

---

## **V. Narrative Body and Effects**

- **Body:**
    
    - May contain any mix of:
        
        - Narrative output: `print "..."` with full interpolation and grammar/template expansion
            
        - State mutations: `set!`, `add!`, etc.
            
        - Choices (inline or block): see below
            
        - Calls to other storylets, events, or threads
            
        - Conditional logic (using canonical prefix `if ... else ...` statements)
            
    - **Canonical example:**
        
        ```sutra
        print "A [adjective] duel breaks out between {agent.name} and {rival.name}!"
        if (gt? agent.rivalry 5) {
          print "{agent.name} fights furiously!"
          set! agent.wounds (+ agent.wounds 2)
        }
        else {
          print "{agent.name} quickly yields."
          set! agent.wounds (+ agent.wounds 1)
        }
        ```
        

---

## **VI. Choices and Outcomes**

- **Choices:**
    
    - May be declared inline within the body or as a block.
        
    - Use block structure for clarity:
        
        ```sutra
        choices {
          "Attack" {
            print "You strike!"
            set! agent.hp (- agent.hp 2)
          }
          "Yield" {
            print "You surrender."
            set! agent.reputation (- agent.reputation 1)
          }
        }
        ```
        
    - Eligibility and gating for choices are expressed as `when` (prefix boolean) inside each choice block.
        
    - Choices may also use resource gates, requires, etc., per Tier 2/3 macro specs.
        

---

## **VII. Pool Membership and Selection**

- **Storylets are gathered into pools by:**
    
    - Tag: `pool (tag social)`
        
    - Explicit list: `pool { storylet ... }`
        
    - Metadata: Any prefix key-value fields (e.g., `region`, `faction`, etc.)
        
- **Pools are filtered and sampled by eligibility and weight, then fired for the current context (agent, player, etc.)**
    
- **Selection Macros:**
    
    - `select from pool ...`
        
    - `random from pool ...`
        
    - `sample N from pool ...`
        
    - `shuffle pool ...`
        

---

## **VIII. Contextual Evaluation**

- **All expressions in eligibility, weight, when, etc., are evaluated in the context of the current agent/player/world as defined by the simulation loop.**
    
- E.g., within a `for-each world.npcs agent { ... }` block, `agent` refers to the current NPC.
    

---

## **IX. Integration with Dynamic Text and Grammar**

- **All narrative and print statements support full `{prefix-expr}` interpolation and `[slot]` grammar/template expansion.**
    
- E.g.:
    
    ```sutra
    print "The [adjective] duel begins! {agent.name} faces {rival.name}."
    ```
    

---

## **X. Thread/Module Integration**

- **Storylets can be fired from within threads or as part of agent/world simulation.**
    
- May “call” other threads, steps, or storylets for complex, modular control flow.
    

---

## **XI. Example: Storylet with Full Features**

```sutra
storylet "feast" {
  tag social
  region castle
  weight (if agent.hungry 10 else 2)
  when (and agent.invited (not (seen "feast")))
  history

  print "{agent.name} attends a lavish feast in the [place]!"
  choices {
    "Eat heartily" {
      print "The food is delicious."
      set! agent.hunger 0
    }
    "Decline wine" {
      print "{agent.name} stays sharp and sober."
      set! agent.wisdom (+ agent.wisdom 1)
    }
  }
}
```

---

## **XII. System Guarantees and Principles**

- **Encapsulation:**
    
    - All storylet state/effects are local to the agent/player/context unless explicitly exported.
        
- **Uniform Syntax:**
    
    - Always prefix notation for all expressions, logic, and conditions.
        
- **Composable:**
    
    - Storylets can be organized, reused, and pooled flexibly.
        
- **Replayable/Emergent:**
    
    - Weighted selection, gating, and history tracking provide infinite variety and author-driven emergence.
        

---

## **XIII. Integration Points and Extension**

- **Storylets are universally composable:**
    
    - Used in pools, threads, world simulation ticks, agent event draws, and more.
        
    - Can reference, trigger, or branch to other modules/content.
        
    - Eligible for all Tier 3+ dynamic/pool/grammar systems.
        

---
